module Main where

record :: { nested :: { value :: Int } }
record = { nested: { value: 42 } }

access = record.nested.value
--              $      $
