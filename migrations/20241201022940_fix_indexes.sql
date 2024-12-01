drop index report_todo;
create index report_todo on report (id) where processed_at is null;

-- don't think i need this
drop index report_processed_at;
