module Main where

select :: { first :: Int, nested :: { second :: String } } -> String
select { first, nested: { second } } = second
