update report set processed_at = null, processing_error = null
  where processing_error
  like 'invalid type: null, expected u32 at line 1 column %';
