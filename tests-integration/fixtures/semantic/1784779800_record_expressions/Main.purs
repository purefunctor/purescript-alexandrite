module Main where

value :: Int
value = 42

empty :: {}
empty = {}

explicit :: { z :: Int, a :: String }
explicit = { z: 1, a: "text" }

punned :: { value :: Int }
punned = { value }

nested :: { array :: Array Int, record :: { boolean :: Boolean } }
nested = { array: [1, 2], record: { boolean: true } }

consume :: { value :: Int } -> Int
consume { value: inner } = inner

application :: Int
application = consume { value: 1 }
