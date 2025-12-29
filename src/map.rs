//! Utilities to create maps to visualize data.

use crate::MapArgs;
use anyhow::Result;
use approx::{abs_diff_eq, relative_eq};
use futures::{Stream, TryStreamExt};
use geo::{BooleanOps, Translate};
use geo_types::{Coord, LineString, Polygon};
use geojson::Geometry;
use h3o::{CellIndex, DirectedEdgeIndex};
use sqlx::{query_scalar, PgPool};
use std::array::from_fn;
use std::cell::Cell;
use std::collections::VecDeque;
use std::io::{stdout, Write};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::thread;

const EPSILON: f64 = 0.00000000001;

/// A h3 cell including relevant information about its edges/neighbors.
#[derive(Clone)]
struct CellData {
    /// H3 cell index for this cell.
    value: u64,

    /// The edges of this cell. There are 12 pentagon cells which have 5 edges, those will have the
    /// 6th edge marked as consumed right from the start. This is faster than having a variable
    /// number of edges because it allows the compiler to optimize for fixed length loops.
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
        // Will normally return 6 edges, but 5 when this cell is a pentagon. In the latter case
        // we mark the 6th cell consumed below.
        let mut edges = cell.edges();

        CellData {
            value: cell.into(),
            edges: from_fn(|_i| match edges.next() {
                None => EdgeData::dummy(),
                Some(e) => EdgeData {
                    destination: e.destination().into(),
                    line: e.into(),
                    consumed: Cell::new(false),
                },
            }),
            consumed: Cell::new(false),
        }
    }

    /// Check if the given cell index is a neighbor of `self`. Because we all edges are kept in
    /// memory this is a fast check that does not involve any calculations.
    ///
    /// If [other] is found the matching edge is marked as [used](EdgeData.consumed).
    fn neighbors_with(&mut self, other: u64) -> bool {
        for i in 0..6 {
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
        for i in 0..6 {
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
    #[inline]
    fn is_enclosed(&self) -> bool {
        for i in 0..6 {
            if !self.edges[i].consumed.get() {
                return false;
            }
        }
        self.mark_consumed();
        true
    }
}

/// Edge of a cell.
///
/// Stores the destination cell for neighbor checking and the [EdgeLine] to produce coordinates
/// when this edge ends up being part of a polygon.
#[derive(Debug, Clone, PartialEq)]
struct EdgeData {
    /// The h3 cell of the destination of this edge.
    destination: u64,

    /// The h3 edge index of this edge.
    line: u64,

    /// Flag to indicate this edge is fully processed and no longer relevant.
    /// We use this flag because it's faster than actually removing the cell from a vector and more
    /// convenient than using `Option` in vectors because it allows for interior mutability.
    consumed: Cell<bool>,
}

impl EdgeData {
    /// Creates an empty edge, only used to fill dummy edges for pentagons.
    const fn dummy() -> Self {
        EdgeData {
            destination: 0,
            line: 0,
            consumed: Cell::new(true),
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

/// Edge of a cell, converted to coordinates.
#[derive(PartialEq)]
struct EdgeCoords {
    /// Start coordinates of this edge.
    start: Coord,

    /// End coordinates of this edge.
    end: Coord,

    /// Flag to indicate this edge is fully processed and no longer relevant.
    /// We use this flag because it's faster than actually removing the cell from a vector and more
    /// convenient than using `Option` in vectors because it allows for interior mutability.
    consumed: Cell<bool>,
}

/// A cluster of adjacent h3 cells that can eventually be turned into a polygon.
struct Cluster(Vec<CellData>);

impl Cluster {
    /// Creates a new filled with a single initial cell.
    fn new(initial: u64) -> Self {
        let initial = CellData::new(initial);
        let mut result = Cluster(Vec::with_capacity(20));
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
                            for j in 0..6 {
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

    /// Convert the current cluster into a polygon.
    ///
    /// Will always return at least one polygon, but may return a second polygon if this cluster
    /// crosses the antimeridian, in which case we split the polygon across the antimeridian.
    fn into_polygon(self) -> Vec<Polygon> {
        // We collect all free edges, e.g. those directed towards a cell that's not part of the
        // cluster, as those are all going to become a line in the polygon.

        let edges = self
            .0
            .iter()
            .flat_map(|c| c.edges.iter().filter(|e| !e.consumed.get()))
            .map(|e| {
                let boundary = DirectedEdgeIndex::try_from(e.line).unwrap().boundary();
                EdgeCoords {
                    start: (*boundary.first().unwrap()).into(),
                    end: (*boundary.last().unwrap()).into(),
                    consumed: Cell::new(false),
                }
            })
            .collect::<Vec<_>>();

        let is_crossing = edges.iter().any(|e| {
            // A check > 179 for cells at the zoom levels we use, and no need to check -179 as
            // there will always be an edge in both directions.
            e.start.x > 179.0 && e.start.x.is_sign_negative() != e.end.x.is_sign_negative()
        });

        // We might have holes and need the outer ring to be output first. So we start at the most
        // northern edge as we can be sure this is on the outer ring.
        let first = edges
            .iter()
            .max_by(|a, b| a.start.y.total_cmp(&b.start.y))
            .unwrap();

        let mut points = Vec::<Coord>::with_capacity(edges.len());
        let mut current = first;
        loop {
            points.push(current.end);
            // No need to check consumed, there can only be one other edge at the same point (or
            // a cell would be enclosed) and not checking seems faster.
            current = match edges
                .iter()
                .find(|x| abs_diff_eq!(x.start, current.end, epsilon = EPSILON))
            {
                Some(x) => x,
                None => {
                    break;
                }
            };
            current.consumed.set(true);
            if current == first {
                points.push(current.end);
                break;
            }
        }
        let mut outer = points;

        // Now we need to do the inner loops, if any. We consumed the edges we used, so whatever is
        // left is part of some inner loop. Per geojson we need to do the inner polygons in the
        // reverse (clockwise) order, but since we are tracking 'filled' part of the polygon this
        // happens automatically.
        let mut inners = Vec::<Vec<Coord>>::with_capacity(10);
        loop {
            // First collecting the remaining edges is faster then a filter iterator.
            let edges = edges
                .iter()
                .filter(|x| !x.consumed.get())
                .collect::<Vec<_>>();
            let first = edges.first();
            match first {
                None => break,
                Some(first) => {
                    let mut points = Vec::<Coord>::with_capacity(100);
                    let mut current = first;
                    loop {
                        points.push(current.end);

                        // No need to check consumed, there can only be one other edge at the same point (or
                        // a cell would be enclosed) and not checking seems faster.
                        current = edges
                            .iter()
                            .find(|x| relative_eq!(x.start, current.end, epsilon = EPSILON))
                            .unwrap();
                        current.consumed.set(true);
                        if current == first {
                            points.push(current.end);
                            break;
                        }
                    }
                    inners.push(points);
                }
            }
        }

        if is_crossing {
            // When crossing the antimeridian we 'adjust' points that are negative to become over
            // 180º, which gives us a polygon with the correct shape, but with invalid coordinates.
            // We get the intersection and difference with the valid earth coordinates which splits
            // the polygon into polygons for both sides of the antimeridian. To make them valid
            // again we convert the coordinates > 180º back to their original (negative) value.
            // Could probably be optimized, but this case should be really rare.
            outer.iter_mut().for_each(|o| {
                if o.x.is_sign_negative() {
                    o.x += 360.0
                }
            });
            for inner in &mut inners {
                inner.iter_mut().for_each(|o| {
                    if o.x.is_sign_negative() {
                        o.x += 360.0
                    }
                });
            }

            let inners: Vec<_> = inners.into_iter().map(LineString::new).collect();
            let result = Polygon::new(LineString::new(outer), inners);

            let split = Polygon::new(
                LineString::new(vec![
                    Coord::<f64>::from((-180.0, -90.0)),
                    Coord::<f64>::from((180.0, -90.0)),
                    Coord::<f64>::from((180.0, 90.0)),
                    Coord::<f64>::from((-180.0, 90.0)),
                    Coord::<f64>::from((-180.0, -90.0)),
                ]),
                Vec::new(),
            );
            let inside = result.intersection(&split);
            let mut outside = result.difference(&split);
            outside.translate_mut(-360.0, 0.0);
            inside.into_iter().chain(outside).collect::<Vec<_>>()
        } else {
            let inners: Vec<_> = inners.into_iter().map(LineString::new).collect();
            let result = Polygon::new(LineString::new(outer), inners);
            vec![result]
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
    let q = query_scalar!("select h3 from map order by h3")
        .fetch(&pool)
        .map_ok(convert);

    // We use a separate thread to convert the clusters of cells into polygons and print them to
    // stdout. A channel is used to send clusters to this thread, giving us some parallelization.
    // This thread must be running before process is called.
    let mut out = stdout();
    let (cluster_tx, cluster_rx) = sync_channel::<Cluster>(50);

    let writer_thread = thread::spawn(move || {
        while let Some(cluster) = cluster_rx.iter().next() {
            let polygons = cluster.into_polygon();
            for poly in polygons {
                writeln!(out, "{}", Geometry::new((&poly).into())).unwrap();
            }
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
        let polygons = cluster.into_polygon();
        assert_eq!(polygons.len(), 1);
        assert_eq!(
            polygons[0],
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
        let polygons = cluster.into_polygon();
        assert_eq!(polygons.len(), 1);
        assert_eq!(
            polygons[0],
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

    #[test]
    fn simple_antimeridian() {
        let cells = [0x88719292b7fffff, 0x88719292b5fffff];
        let mut cluster = Cluster::new(*cells.first().unwrap());
        for cell in cells.iter().skip(1) {
            assert!(cluster.add_when_neighboring(*cell));
        }
        let polygons = cluster.into_polygon();
        assert_eq!(polygons.len(), 2);
        assert_eq!(
            polygons[0],
            Polygon::new(
                LineString(vec![
                    Coord::from((179.9999999226187, 5.570534706115723)),
                    Coord::from((179.99996320615753, 5.57046914100647)),
                    Coord::from((179.9999999226187, 5.570393085479736)),
                    Coord::from((179.9999999226187, 5.570534706115723)),
                ]),
                Vec::new()
            )
        );
        assert_eq!(
            polygons[1],
            Polygon::new(
                LineString(vec![
                    Coord::from((-180.0000000773813, 5.570534706115723)),
                    Coord::from((-180.0000000773813, 5.570393085479736)),
                    Coord::from((-179.99805315126434, 5.566359519958496)),
                    Coord::from((-179.99400098909393, 5.565949201583862)),
                    Coord::from((-179.9920173465158, 5.56183934211731)),
                    Coord::from((-179.98796470750824, 5.561428546905518)),
                    Coord::from((-179.9858957110788, 5.565127849578857)),
                    Coord::from((-179.9878795920755, 5.569237947463989)),
                    Coord::from((-179.99193223108307, 5.569648265838623)),
                    Coord::from((-179.99391611207977, 5.573758125305176)),
                    Coord::from((-179.99796851266876, 5.57416844367981)),
                    Coord::from((-180.0000000773813, 5.570534706115723)),
                ]),
                Vec::new()
            )
        );
    }

    #[test]
    fn complex_antimeridian() {
        let cells = [
            0x880d9ecebdfffff,
            0x880d9ecea3fffff,
            0x880d9ecea7fffff,
            0x880d9ecea5fffff,
            0x880d9ecc53fffff,
            0x880d9ecc5bfffff,
            0x880d9ecee5fffff,
            0x880d9ecee1fffff,
            0x880d9eceebfffff,
            0x880d9ecec7fffff,
            0x880d9ecec3fffff,
            0x880d9eceddfffff,
            0x880d9ec537fffff,
            0x880d9ec535fffff,
            0x880d9ec523fffff,
            0x880d9ec521fffff,
            0x880d9ec525fffff,
            0x880d9ece19fffff,
            0x880d9ece1dfffff,
            0x880d9ece15fffff,
            0x880d9ece3bfffff,
            0x880d9ece33fffff,
        ];
        let mut cluster = Cluster::new(*cells.first().unwrap());
        for cell in cells.iter().skip(1) {
            assert!(cluster.add_when_neighboring(*cell));
        }
        let polygons = cluster.into_polygon();
        assert_eq!(polygons.len(), 3);

        assert_eq!(
            polygons[0],
            Polygon::new(
                LineString(vec![
                    Coord::from((179.93330854833243, 65.08215880393982)),
                    Coord::from((179.92669052541373, 65.07793855667114)),
                    Coord::from((179.93197912633536, 65.0735604763031)),
                    Coord::from((179.94388288915275, 65.073401927948)),
                    Coord::from((179.94916696012137, 65.0690233707428)),
                    Coord::from((179.94255012929557, 65.06480431556702)),
                    Coord::from((179.94783276975272, 65.06042623519897)),
                    Coord::from((179.9412185615313, 65.05620741844177)),
                    Coord::from((179.9464995330584, 65.05182957649231)),
                    Coord::from((179.95839209020255, 65.05167007446289)),
                    Coord::from((179.96366877019523, 65.04729199409485)),
                    Coord::from((179.97555846631644, 65.04713129997253)),
                    Coord::from((179.9808308547747, 65.04275274276733)),
                    Coord::from((179.99271768987296, 65.04259061813354)),
                    Coord::from((179.9993364280474, 65.04680681228638)),
                    Coord::from((179.99999994695304, 65.04679775238037)),
                    Coord::from((179.99999994695304, 65.05496382713318)),
                    Coord::from((179.99406666219352, 65.05118584632874)),
                    Coord::from((179.98217553556083, 65.0513482093811)),
                    Coord::from((179.9769017165911, 65.05572700500488)),
                    Coord::from((179.96500772893546, 65.05588793754578)),
                    Coord::from((179.95972938001273, 65.06026649475098)),
                    Coord::from((179.9663478797686, 65.06448459625244)),
                    Coord::from((179.9610678619158, 65.06886339187622)),
                    Coord::from((179.96768874585746, 65.07308173179626)),
                    Coord::from((179.97959107816337, 65.07292008399963)),
                    Coord::from((179.98621601522086, 65.07713747024536)),
                    Coord::from((179.99811953961967, 65.07697463035583)),
                    Coord::from((179.99999994695304, 65.07817077636719)),
                    Coord::from((179.99999994695304, 65.08590722084045)),
                    Coord::from((179.99947256505607, 65.08557176589966)),
                    Coord::from((179.98756474912284, 65.08573508262634)),
                    Coord::from((179.98093718946097, 65.08151745796204)),
                    Coord::from((179.96903080403922, 65.08167934417725)),
                    Coord::from((179.9624072974932, 65.07746076583862)),
                    Coord::from((179.95050210416434, 65.07762098312378)),
                    Coord::from((179.94521636426566, 65.08200001716614)),
                    Coord::from((179.95183820188163, 65.0862193107605)),
                    Coord::from((179.9465507930529, 65.09059858322144)),
                    Coord::from((179.95317525327323, 65.09481811523438)),
                    Coord::from((179.94788617551444, 65.09919762611389)),
                    Coord::from((179.95451349675773, 65.10341715812683)),
                    Coord::from((179.9664310878527, 65.10325646400452)),
                    Coord::from((179.97306222379325, 65.10747528076172)),
                    Coord::from((179.9849812453997, 65.10731267929077)),
                    Coord::from((179.9916164344561, 65.11153078079224)),
                    Coord::from((179.99999994695304, 65.11141538619995)),
                    Coord::from((179.99999994695304, 65.12003350257874)),
                    Coord::from((179.99296898305533, 65.12013030052185)),
                    Coord::from((179.98633117139457, 65.11591219902039)),
                    Coord::from((179.9744078582537, 65.11607480049133)),
                    Coord::from((179.96777409970878, 65.11185598373413)),
                    Coord::from((179.95585245549796, 65.1120171546936)),
                    Coord::from((179.94922275006888, 65.1077971458435)),
                    Coord::from((179.93730229795096, 65.1079568862915)),
                    Coord::from((179.93067664563773, 65.10373616218567)),
                    Coord::from((179.93597001493094, 65.09935688972473)),
                    Coord::from((179.92934698522208, 65.09513664245605)),
                    Coord::from((179.93463868558524, 65.0907576084137)),
                    Coord::from((179.92801827848075, 65.08653736114502)),
                    Coord::from((179.93330854833243, 65.08215880393982)),
                ]),
                Vec::new(),
            )
        );

        assert_eq!(
            polygons[1],
            Polygon::new(
                LineString(vec![
                    Coord::from((-180.00000005304696, 65.05496382713318)),
                    Coord::from((-180.00000005304696, 65.04679775238037)),
                    Coord::from((-179.98877530634286, 65.04664301872253)),
                    Coord::from((-179.98215251505258, 65.05085825920105)),
                    Coord::from((-179.97026305734994, 65.05069327354431)),
                    Coord::from((-179.9636362129438, 65.05490756034851)),
                    Coord::from((-179.96890049517037, 65.05928826332092)),
                    Coord::from((-179.98079424440743, 65.05945348739624)),
                    Coord::from((-179.9874196583021, 65.05523824691772)),
                    Coord::from((-179.99931197702767, 65.05540204048157)),
                    Coord::from((-180.00000005304696, 65.05496382713318)),
                ]),
                Vec::new()
            )
        );

        assert_eq!(
            polygons[2],
            Polygon::new(
                LineString(vec![
                    Coord::from((-180.00000005304696, 65.08590722084045)),
                    Coord::from((-180.00000005304696, 65.07817077636719)),
                    Coord::from((-179.995251470207, 65.08119106292725)),
                    Coord::from((-179.98334627687814, 65.08102655410767)),
                    Coord::from((-179.97671323358895, 65.08524250984192)),
                    Coord::from((-179.9648068481672, 65.08507633209229)),
                    Coord::from((-179.95816975176217, 65.08929133415222)),
                    Coord::from((-179.94626193582894, 65.08912372589111)),
                    Coord::from((-179.93962078630807, 65.09333801269531)),
                    Coord::from((-179.94488864481332, 65.09772062301636)),
                    Coord::from((-179.93824487268807, 65.10193514823914)),
                    Coord::from((-179.94351463854196, 65.10631823539734)),
                    Coord::from((-179.95543079912545, 65.1064863204956)),
                    Coord::from((-179.96070461809518, 65.11086916923523)),
                    Coord::from((-179.97262363970162, 65.1110360622406)),
                    Coord::from((-179.97790198862435, 65.11541843414307)),
                    Coord::from((-179.98982363283517, 65.11558413505554)),
                    Coord::from((-179.99510627329232, 65.11996626853943)),
                    Coord::from((-180.00000005304696, 65.12003350257874)),
                    Coord::from((-180.00000005304696, 65.11141538619995)),
                    Coord::from((-179.996463113426, 65.11136674880981)),
                    Coord::from((-179.9911821418989, 65.10698509216309)),
                    Coord::from((-179.97926455080392, 65.10681986808777)),
                    Coord::from((-179.97398787081124, 65.10243773460388)),
                    Coord::from((-179.96207314073922, 65.10227108001709)),
                    Coord::from((-179.95680075228097, 65.09788870811462)),
                    Coord::from((-179.96344047129037, 65.09367346763611)),
                    Coord::from((-179.97535090982797, 65.09383964538574)),
                    Coord::from((-179.98198657572152, 65.08962368965149)),
                    Coord::from((-179.99389582216622, 65.08978867530823)),
                    Coord::from((-180.00000005304696, 65.08590722084045)),
                ]),
                Vec::new()
            )
        );
    }
}
