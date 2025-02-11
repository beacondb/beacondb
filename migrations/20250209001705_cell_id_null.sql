update report set processed_at = null, processing_error = null
  where processing_error
  like 'invalid type: null, expected u64 at line 1 column %';
