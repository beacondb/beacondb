# Reprocessing data

As BeaconDB's algorithms are not yet stablised, instance owners may need to reprocess submitted data as part of new releases. There are two ways to do so, depending on how large your database is. For databases with less than 10 million reports, marking reports as unprocessed in Postgres is the easiest solution. For larger databases, storing reports in Postgres becomes infeasible and instead should be [exported as JSON](./exporting-data.md) before being reprocessed.

> [!WARNING]
>
> Existing positioning data will need to be wiped, meaning your instance won't be able to answer queries until data is reprocessed. If this is a problem, consider copying reports to a second database to be reprocessed separate from your production database.

## Mark reports as unprocessed in Postgres (less than 10M)

```sql
-- mark reports as unprocessed
update report set processed_at = null, processing_error = null;

-- wipe existing positioning data
truncate table cell;
truncate table wifi;
truncate table bluetooth;
truncate table map;
```

The database can then be processed as usual (`beacondb process`).

## Reprocessing exported data (>10M)

Data [exported from BeaconDB](./exporting-data.md) can be reprocessed using `beacondb bulk process`.

Bulk-processing will write data to the `cell`, `wifi`, `bluetooth` and `map` tables - exactly the same as `beacondb process` but reports are read from standard input instead of Postgres.

You'll likely want to use an empty database for reprocessing, as `bulk process` will not delete existing data. You can run it multiple times to process multiple exports.

```sh
# read and process multiple exports
zstdcat 2026-01-reports.jsonl.zst | beacondb bulk process
zstdcat 2026-02-reports.jsonl.zst | beacondb bulk process
```
