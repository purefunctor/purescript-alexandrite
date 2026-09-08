module Main where

identity :: forall a. a -> a
identity value = value

record :: { first :: Int, nested :: { value :: Int }, transform :: Int -> Int }
record = { first: 1, nested: { value: 2 }, transform: identity }

leaf = record { first = 2 }

typeChanging = record { first = "two" }

multiple = record { first = 2, transform = identity }

nested = record { nested { value = 3 } }

accessValue = record { first = record.nested.value }

applicationBase = (identity record) { first = 2 }

argument = identity record { first = 2 }
