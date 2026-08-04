module Main where

captureRenamedReference original = (\renamed -> original) 0
--                      /

captureExistingReference renamed =
  let original = 0
--      /
  in renamed + original

captureLetReference original =
--                    /
  let renamed = 0
  in original + renamed

capturePunReference { renamed } =
  let original = 0
--      /
  in renamed + original

captureNamedBinder original@{ renamed } = original
--                 /

captureRecordPun renamed { original } = original
--                          /

preserveNestedReference original = (\renamed -> renamed) original
--                      /

rejectUnusedParent renamed =
  let original = 0
--      /
  in original

captureLambda renamed = (\original -> renamed + original) 0
--                        /

captureCase renamed value = case value of original -> renamed + original
--                                        /

captureDo renamed = do
  original <- renamed
--/
  original

captureAdo renamed = ado
  original <- renamed
--/
  in original
