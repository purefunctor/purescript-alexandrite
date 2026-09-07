module Main where

record :: { nested :: { value :: Int }, value :: Int }
record = { nested: { value: 1 }, value: 2 }

missingValue = record { value = }

nestedMissingValue = record { nested { value = } }

validSibling = record { nested { value = }, value = 3 }
