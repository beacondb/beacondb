# Exporting data

BeaconDB supports exporting submitted data as `.jsonl` ([JSON Lines](https://jsonlines.org/)), as having large amount of data sitting in Postgres is not ideal, especially for larger databases (>10M reports). This plaintext format is easy to use in other programs for debugging and analysis.

If you're exporting data for debugging or analysis, you can run `beacondb bulk export` to export data as JSON on standard output. The `bulk export` command only reads data, and does not modify or delete any data in Postgres. The guide below shows how to safely export data out of Postgres.

There is no built-in support for importing raw report data back into Postgres once exported, but all report metadata stored in Postgres is exported (shown below). [Data can still be reprocessed using `beacondb bulk process`.](./reprocessing.md)

```jsonc
// (this report is pretty-printed here, but would really be one line)
{
  "id": 321012345,
  "submitted_at": "2026-06-01T00:00:00.123456Z",
  "user_agent": "NeoStumbler/62",
  "raw": {
    "cellTowers": [ .. ],
    "position": { .. },
    "timestamp": 1780272000,
    "wifiAccessPoints": [ .. ]
  }
}
```

## Moving reports from Postgres to JSON

This guide includes various checks that are useful to those who are exporting data from a production database that may be actively receiving data from contributors.

```sql
-- save these values for verifying below
select count(*) from report;
select max(id) from report; -- YOUR_MAX_ID
```

```sh
# export data as compressed JSON
beacondb bulk export | zstd -T0 -9 > $(date -I)-beacondb-reports.jsonl.zst

# check:
# should at least be count(*)
zstdcat XXXX-XX-XX-beacondb-reports.jsonl.zst | wc -l
# max(id) should exist
zstdcat XXXX-XX-XX-beacondb-reports.jsonl.zst | grep '"id":YOUR_MAX_ID,'
```

```sql
-- check equal to count(*) earlier
select count(*) from report where id <= YOUR_MAX_ID;

-- finally, removes the exported reports from postgres
delete from report where id <= YOUR_MAX_ID;
```
