module Main where

import Library.Values as Library.Values
--       $       $

qualifiedType :: Library.Values.Value
--                                $
qualifiedType = Library.Values.value
--                               $

qualifiedOperator = Library.Values.(+)
--                                  $

endBoundary = Library.Values.value
--                                $

record = { value: 42 }

recordAccess = record.value
--                   $
