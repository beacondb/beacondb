//! Utilities to create maps to visualize data.

use crate::MapArgs;
use anyhow::Result;
use approx::{abs_diff_eq, relative_eq};
use futures::{Stream, TryStreamExt};
use geo_types::{Coord, LineString, Polygon};
use geojson::Geometry;
use h3o::{CellIndex, DirectedEdgeIndex};
use sqlx::{query_scalar, PgPool};
use std::array::from_fn;
use std::cell::Cell;
use std::collections::VecDeque;
use std::future::Ready;
use std::io::{stdout, Write};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::{future, thread};

/// A h3 cell including relevant information about its edges/neighbors.
#[derive(Clone)]
struct CellData {
    /// H3 cell index for this cell.
    value: u64,

    /// The number of edges in this cell. Normally 6, but 5 if this is a pentagon cell.
    edge_cnt: usize,

    /// The edges of this cell. Only the first [edge_cnt] entries in this array are valid.
    edges: [EdgeData; 6],

    /// Flag to indicate this cell is fully processed and no longer relevant.
    /// We use this flag because it's faster than actually removing the cell from a vector and more
    /// convenient than using `Option` in vectors because it allows for interior mutability.
    consumed: Cell<bool>,
}

impl CellData {
    /// Creates a new h3 CellData from the given index.
    ///
    /// Check for pentagons and loads all edges, initializes [consumed] as false.
    ///
    /// Panics if the index isn't a valid cell index.
    fn new(index: u64) -> Self {
        let cell = CellIndex::try_from(index).unwrap();
        let mut edges = cell.edges();

        CellData {
            value: cell.into(),
            edges: from_fn(|_i| match edges.next() {
                None => EdgeData::default(),
                Some(e) => EdgeData {
                    destination: e.destination().into(),
                    line: Cell::new(EdgeLine::Index(e)),
                    consumed: Cell::new(false),
                },
            }),
            edge_cnt: if cell.is_pentagon() { 5 } else { 6 },
            consumed: Cell::new(false),
        }
    }

    /// Check if the given cell index is a neighbor of `self`. Because we all edges are kept in
    /// memory this is a fast check that does not involve any calculations.
    ///
    /// If [other] is found the matching edge is marked as [used](EdgeData.consumed).
    fn neighbors_with(&mut self, other: u64) -> bool {
        for i in 0..self.edge_cnt {
            if self.edges[i].destination == other {
                self.edges[i].mark_consumed();
                return true;
            }
        }
        false
    }

    /// Marks the edge to a specific neighbor as consumed.
    ///
    /// Needed when we determined we are neighboring another cell, but did so by calling
    /// [is_neighbor_with] on the other cell.
    fn mark_neighbor_consumed(&self, other: u64) {
        for i in 0..self.edge_cnt {
            if self.edges[i].destination == other {
                self.edges[i].mark_consumed();
                break;
            }
        }
    }

    /// Mark this entire cell as being consumed.
    ///
    /// This can, but doesn't have to be, done when all edges are consumed in which case we can
    /// just skip this entire cell when processing.
    #[inline]
    fn mark_consumed(&self) {
        self.consumed.set(true);
    }

    /// Checks if all edges are consumed and marks this cell as consumed if that is the case.
    fn is_enclosed(&self) -> bool {
        for i in 0..self.edge_cnt {
            if !self.edges[i].consumed.get() {
                return false;
            }
        }
        self.mark_consumed();
        true
    }
}

/// Enum representing an edge.
///
/// Initially initialized using a h3 DirectedEdgeIndex and replaced with the start and end
/// coordinates once those are needed. This allows us to lazy load the coordinates saving the
/// required calculations if we never end up using the edge.
#[derive(Debug, Copy, Clone, PartialEq)]
enum EdgeLine {
    Index(DirectedEdgeIndex),
    Coords(Coord, Coord),
}

/// Edge of a cell.
///
/// Stores the destination cell for neighbor checking and the [EdgeLine] to produce coordinates
/// when this edge ends up being part of a polygon.
#[derive(Debug, Clone, PartialEq)]
struct EdgeData {
    /// The h3 cell of the destination of this edge.
    destination: u64,

    line: Cell<EdgeLine>,

    /// Flag to indicate this edge is fully processed and no longer relevant.
    /// We use this flag because it's faster than actually removing the cell from a vector and more
    /// convenient than using `Option` in vectors because it allows for interior mutability.
    consumed: Cell<bool>,
}

impl EdgeData {
    /// Creates an empty edge, only used to fill dummy edges for pentagons.
    const fn default() -> Self {
        EdgeData {
            destination: 0,
            line: Cell::new(EdgeLine::Coords(
                Coord { x: 0.0, y: 0.0 },
                Coord { x: 0.0, y: 0.0 },
            )),
            consumed: Cell::new(false),
        }
    }

    /// Get the start coordinates of this edge.
    ///
    /// This will calculate the coordinates (both start and end) the first time they are needed and
    /// keep them for future use.
    #[inline]
    fn start(&self) -> Coord {
        match self.line.get() {
            EdgeLine::Index(e) => {
                let b = e.boundary();
                let first = (*b.first().unwrap()).into();
                self.line
                    .set(EdgeLine::Coords(first, (*b.last().unwrap()).into()));
                first
            }
            EdgeLine::Coords(s, _) => s,
        }
    }

    /// Get the end coordinates of this edge.
    ///
    /// This will calculate the coordinates (both start and end) the first time they are needed and
    /// keep them for future use.
    #[inline]
    fn end(&self) -> Coord {
        match self.line.get() {
            EdgeLine::Index(e) => {
                let b = e.boundary();
                let last = (*b.last().unwrap()).into();
                self.line
                    .set(EdgeLine::Coords((*b.first().unwrap()).into(), last));
                last
            }
            EdgeLine::Coords(_, e) => e,
        }
    }

    /// Marks this edge consumed excluding this from future calculations. An edge is considered
    /// consumed when during clustering we determine this is an edge between two adjacent cells or
    /// if it isn't if it has been added to the polygon. Both cases mean we don't have to consider
    /// this edge any further.
    #[inline]
    fn mark_consumed(&self) {
        self.consumed.set(true);
    }
}

/// A cluster of adjacent h3 cells that can eventually be turned into a polygon.
struct Cluster(Vec<CellData>);

impl Cluster {
    /// Creates a new filled with a single initial cell.
    fn new(initial: u64) -> Self {
        let initial = CellData::new(initial);
        let mut result = Cluster(Vec::with_capacity(10));
        result.0.push(initial);
        result
    }

    /// Check if the given cell is adjacent to any other cell in this cluster and adds it if it is.
    ///
    /// Also marks all touching edges as consumed so we know we can ignore those. As a slight
    /// optimization the new cell isn't added if it would be fully enclosed immediately, in that
    /// case we don't need it as all.
    fn add_when_neighboring(&mut self, cell: u64) -> bool {
        let mut new: Option<CellData> = None;

        for c in 0..self.0.len() {
            if !self.0[c].consumed.get() && self.0[c].neighbors_with(cell) {
                if new.is_none() {
                    new = Some(CellData::new(cell));
                }
                if let Some(ref mut new) = new {
                    new.mark_neighbor_consumed(self.0[c].value);
                }
                self.0[c].is_enclosed();
            }
        }

        if let Some(new) = new {
            if !new.is_enclosed() {
                self.0.push(new);
            }
            true
        } else {
            false
        }
    }

    /// Merge two adjacent clusters.
    ///
    /// We pass the cell that cause both clusters to be joined, so we know we only need to update
    /// the edges related to that specific cell.
    fn merge(&mut self, other: Cluster, connecting_cell: u64) {
        for i in 0..other.0.len() {
            if !other.0[i].consumed.get() {
                if other.0[i].value != connecting_cell {
                    self.0.push(other.0[i].clone())
                } else {
                    for own_cell in &mut self.0 {
                        if !own_cell.consumed.get() && own_cell.value == connecting_cell {
                            for j in 0..own_cell.edge_cnt {
                                if other.0[i].edges[j].consumed.get() {
                                    own_cell.edges[j].mark_consumed();
                                }
                            }
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Creates an iterator over all edges that have not been consumed yet.
    #[inline]
    fn free_edges(&self) -> impl Iterator<Item = &EdgeData> {
        EdgeIterator::new(self)
    }

    /// Convert the current cluster into a polygon.
    fn into_polygon(self) -> Polygon {
        // We collect all free edges, e.g. those directed towards a cell that's not part of the
        // cluster, as those are all going to become a line in the polygon.
        let edges = self.free_edges();

        // We might have holes and need the outer ring to be output first. So we start at the most
        // northern edge as we can be sure this is on the outer ring.
        let first = edges
            .max_by(|a, b| a.start().y.total_cmp(&b.start().y))
            .unwrap();

        let mut points = Vec::<Coord>::with_capacity(10);
        let mut current = first;
        loop {
            let mut edges = self.free_edges();
            points.push(current.end());
            current = match edges
                .find(|x| abs_diff_eq!(x.start(), current.end(), epsilon = 0.00000000001))
            {
                Some(x) => x,
                None => {
                    break;
                }
            };
            current.mark_consumed();
            if current == first {
                points.push(current.end());
                break;
            }
        }
        let outer = points;

        // Now we need to do the inner loops, if any. We consumed the edges we used, so whatever is
        // left is part of some inner loop. Per geojson we need to do the inner polygons in the
        // reverse (clockwise) order, but since we are tracking 'filled' part of the polygon this
        // happens automatically.
        let mut inners = Vec::<Vec<Coord>>::with_capacity(10);
        loop {
            let mut edges = self.free_edges();

            let first = edges.next();
            match first {
                None => break,
                Some(first) => {
                    let mut points = Vec::<Coord>::with_capacity(10);
                    let mut current = first;
                    loop {
                        points.push(current.end());
                        let mut edges = self.free_edges();
                        current = edges
                            .find(|x| {
                                relative_eq!(x.start(), current.end(), epsilon = 0.00000000001)
                            })
                            .unwrap();
                        current.mark_consumed();
                        if current == first {
                            // points.push(current.start());
                            points.push(current.end());
                            break;
                        }
                    }
                    inners.push(points);
                }
            }
        }

        let inners: Vec<_> = inners.into_iter().map(LineString::new).collect();
        Polygon::new(LineString::new(outer), inners)
    }
}

/// Custom iterator that loops over all cells and edges in a cluster returning only the cells that
/// have not been consumed yet.
///
/// Customized to be able to mark cells as consumed when we find all their edges are consumed which
/// gives us a marginal performance gain.
struct EdgeIterator<'a> {
    cluster: &'a Cluster,
    c: usize,
    e: usize,
}

impl<'a> EdgeIterator<'a> {
    fn new(cluster: &'a Cluster) -> Self {
        EdgeIterator {
            cluster,
            c: 0,
            e: 0,
        }
    }
}

impl<'a> Iterator for EdgeIterator<'a> {
    type Item = &'a EdgeData;

    fn next(&mut self) -> Option<&'a EdgeData> {
        loop {
            if self.c >= self.cluster.0.len() {
                return None;
            }

            if !self.cluster.0[self.c].consumed.get() {
                let new = self.e == 0;
                loop {
                    if self.e >= self.cluster.0[self.c].edges.len() {
                        break;
                    }
                    if !self.cluster.0[self.c].edges[self.e].consumed.get() {
                        let edge = &self.cluster.0[self.c].edges[self.e];
                        self.e += 1;
                        return Some(edge);
                    }
                    self.e += 1;
                }
                if new {
                    self.cluster.0[self.c].mark_consumed();
                }
            }
            self.c += 1;
            self.e = 0;
        }
    }
}

/// Export all h3 cells as individual polygons.
///
/// This exports all cells from the map table as geojson polygons, one per line. This output can
/// then be piped into tippecanoe which can handle 1 geojson Polygon per line. This should perform
/// better than creating a single big MultiPolygon because tippecanoe won't have to parse a big json
/// object.
///
/// Cells are merged into a single polygon with a best effort approach that will merge most, but
/// not all adjacent cells. We keep track of a number of clusters we've come across earlier, which
/// are likely to be somewhat close to each other because we read the cells in index order. Any new
/// cell is checked only against those cluster, not all other cells to keep performance and memory
/// usage at a reasonable level. This can be tuned using [MapArgs.lookback_size].
pub async fn run(pool: PgPool, args: MapArgs) -> Result<()> {
    let q = query_scalar!("select h3 from map_all order by h3")
        .fetch(&pool)
        .map_ok(convert)
        .try_filter(antimeridian_filter);

    // We use a separate thread to convert the clusters of cells into polygons and print them to
    // stdout. A channel is used to send clusters to this thread, giving us some parallelization.
    // This thread must be running before process is called.
    let mut out = stdout();
    let (cluster_tx, cluster_rx) = sync_channel::<Cluster>(50);

    let writer_thread = thread::spawn(move || {
        while let Some(cluster) = cluster_rx.iter().next() {
            writeln!(out, "{}", Geometry::new((&cluster.into_polygon()).into())).unwrap();
        }
    });

    // Merge cells into clusters of cells.
    process(q, args.lookback_size, cluster_tx).await?;

    // We must wait for the writer thread to finish, or we might miss some output.
    writer_thread.join().unwrap();
    Ok(())
}

/// Process a stream of cells merging them into clusters. All clusters we find are send to the
/// [cluster_tx] channel for further processing.
async fn process<T>(mut q: T, lookback_size: usize, cluster_tx: SyncSender<Cluster>) -> Result<()>
where
    T: Stream<Item = Result<u64, sqlx::Error>> + Unpin,
{
    let mut clusters = VecDeque::<Cluster>::with_capacity(lookback_size);

    while let Some(x) = q.try_next().await? {
        let mut added_to = Vec::<usize>::with_capacity(10);

        for i in (0..clusters.len()).rev() {
            if clusters[i].add_when_neighboring(x) {
                added_to.push(i);
            }
        }

        if added_to.len() > 1 {
            let first = added_to.first().unwrap();
            for (i, idx) in added_to.iter().enumerate().skip(1) {
                let merge = clusters.remove(*idx).unwrap();
                clusters.get_mut(*first - i).unwrap().merge(merge, x);
            }
        } else if added_to.is_empty() {
            // We did not add this cell, so it becomes the start of a new cluster
            if clusters.len() == lookback_size {
                let cluster = clusters.pop_front().unwrap();
                cluster_tx.send(cluster).unwrap();
            }

            clusters.push_back(Cluster::new(x));
        }
    }

    // Deal with the remaining clusters
    for cluster in clusters {
        cluster_tx.send(cluster).unwrap();
    }
    Ok(())
}

/// Convert bytes (as read from postgres) into an u64.
#[inline]
fn convert(x: Vec<u8>) -> u64 {
    assert_eq!(x.len(), 8);

    let x: [u8; 8] = x.try_into().unwrap();
    u64::from_be_bytes(x)
}

/// Filters cells that cross the antimeridian.
fn antimeridian_filter(cell: &u64) -> Ready<bool> {
    // FIXME: We should split output polygons at the antimeridian after which this can be removed.
    // If we get a cell which crosses the antimeridian we get lines from -179.x to 179.x degrees
    // which are then interpreted as shape that crosses across the other side of the earth. This
    // results in horizontal lines being drawn across the map.
    // Per geojson spec we should cut those into two shapes to prevent this from happening.
    // See: https://datatracker.ietf.org/doc/html/rfc7946#section-3.1.9
    let s: LineString = CellIndex::try_from(*cell).unwrap().boundary().into();
    let is_crossing = s
        .lines()
        .any(|l| l.start.x.is_sign_negative() != l.end.x.is_sign_negative());
    future::ready(!is_crossing)
}

#[cfg(test)]
mod tests {
    use crate::map::{process, Cluster};
    use futures::stream;
    use geo::Coord;
    use geo_types::{LineString, Polygon};
    use std::sync::mpsc::sync_channel;

    #[tokio::test]
    async fn process_triple_merge() {
        // Three separate cells which are joined together by the last one.
        let cells: [Result<u64, sqlx::Error>; 4] = [
            Ok(0x882ba14733fffff),
            Ok(0x882ba1455bfffff),
            Ok(0x882ba14461fffff),
            Ok(0x882ba14465fffff),
        ];
        let stream = stream::iter(cells);

        let (cluster_tx, cluster_rx) = sync_channel::<Cluster>(50);

        process(stream, 10, cluster_tx).await.unwrap();
        let cluster = cluster_rx.recv().unwrap();
        assert_eq!(cluster.0.len(), 4);
        assert!(cluster.0.iter().any(|c| c.value == 0x882ba14733fffff));
        assert!(cluster.0.iter().any(|c| c.value == 0x882ba1455bfffff));
        assert!(cluster.0.iter().any(|c| c.value == 0x882ba14461fffff));
        assert!(cluster.0.iter().any(|c| c.value == 0x882ba14465fffff));
        // There should be just one cluster generated
        assert!(cluster_rx.recv().is_err());
    }

    #[test]
    fn simple_cluster() {
        // Test cluster on top of Lac Hexagonal
        let cells = [
            0x882ba14465fffff,
            0x882ba14461fffff,
            0x882ba1446dfffff,
            0x882ba14733fffff,
            0x882ba14559fffff,
            0x882ba1455bfffff,
            0x882ba14467fffff,
        ];

        let mut cluster = Cluster::new(*cells.first().unwrap());
        for cell in cells.iter().skip(1) {
            assert!(cluster.add_when_neighboring(*cell));
        }
        let result = cluster.into_polygon();
        assert_eq!(
            result,
            Polygon::new(
                LineString(vec![
                    Coord::from((-72.01068058531455, 47.50426758066505)),
                    Coord::from((-72.01317449786164, 47.49953890080424)),
                    Coord::from((-72.02061441417537, 47.49855497247617)),
                    Coord::from((-72.02310694028151, 47.493826262508925)),
                    Coord::from((-72.01816070992638, 47.490081882768266)),
                    Coord::from((-72.02065300977287, 47.48535354476923)),
                    Coord::from((-72.01570797180803, 47.48160934375985)),
                    Coord::from((-72.00827089336076, 47.48259288568877)),
                    Coord::from((-72.0033273068264, 47.47884826861245)),
                    Coord::from((-71.99589052129716, 47.47983099262631)),
                    Coord::from((-71.99339590378433, 47.48455852655395)),
                    Coord::from((-71.98595799312544, 47.48554062539006)),
                    Coord::from((-71.98346198977721, 47.490268128930836)),
                    Coord::from((-71.98840505539671, 47.494013936021155)),
                    Coord::from((-71.985908824193, 47.498741811384505)),
                    Coord::from((-71.99085308174361, 47.50248779764889)),
                    Coord::from((-71.99829383141038, 47.50150531344075)),
                    Coord::from((-72.00323954212895, 47.50525088350742)),
                    Coord::from((-72.01068058531455, 47.50426758066505)),
                ]),
                Vec::new()
            )
        );
    }

    #[test]
    fn simple_cluster_with_cell_rotation() {
        // These cells are on a crossover point where edges rotate. Used to be an issue in an
        // earlier implementation of the neighbor detection.
        let cells = [
            0x883000a691fffff,
            0x883000a693fffff,
            0x883000a695fffff,
            0x883000a697fffff,
            0x883000a6bbfffff,
            0x8830039649fffff,
            0x883003964bfffff,
        ];

        let mut cluster = Cluster::new(*cells.first().unwrap());
        for cell in cells.iter().skip(1) {
            assert!(cluster.add_when_neighboring(*cell));
        }
        let result = cluster.into_polygon();
        assert_eq!(
            result,
            Polygon::new(
                LineString(vec![
                    Coord::from((122.8457610372325, 39.06100113491305)),
                    Coord::from((122.8416728829429, 39.06268183911169)),
                    Coord::from((122.8383387897483, 39.06019870263979)),
                    Coord::from((122.83909264440105, 39.05603512914009)),
                    Coord::from((122.83575918258151, 39.053552269053945)),
                    Coord::from((122.83651308975003, 39.0493888730055)),
                    Coord::from((122.84060008644289, 39.04770797092555)),
                    Coord::from((122.84135367386305, 39.04354438630302)),
                    Coord::from((122.8454405571582, 39.04186302846133)),
                    Coord::from((122.848774431648, 39.04434535407648)),
                    Coord::from((122.85286178008712, 39.04266363926978)),
                    Coord::from((122.85619649217733, 39.04514597397362)),
                    Coord::from((122.85544364967357, 39.04931029082535)),
                    Coord::from((122.85877899337997, 39.05179290189006)),
                    Coord::from((122.85802620373745, 39.05595739630494)),
                    Coord::from((122.85393769766638, 39.057638913528464)),
                    Coord::from((122.8531845881319, 39.06180321928758)),
                    Coord::from((122.84909596841514, 39.06348428053087)),
                    Coord::from((122.8457610372325, 39.06100113491305)),
                ]),
                Vec::new()
            )
        );
    }
}
