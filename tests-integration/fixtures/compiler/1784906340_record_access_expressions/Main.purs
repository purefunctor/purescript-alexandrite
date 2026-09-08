module Main where

identity :: forall a. a -> a
identity value = value

record :: { function :: Int -> Int, nested :: { value :: Int } }
record = { function: identity, nested: { value: 42 } }

direct = record.nested

chained = record.nested.value

functionPosition = record.function 1

argument = identity record.nested

applicationBase = (identity record).nested.value
