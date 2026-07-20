module Main where

foreign import identity :: forall a. a -> a

checkedArray :: Array Int
checkedArray = [identity 1, 2]

inferredArray = ["one", "two"]

emptyArray = []

checkedRecord :: { number :: Int, text :: String }
checkedRecord = { number: identity 1, text: "value" }

inferredRecord = { array: [1, 2], nested: { value: true } }

punValue = 42

punnedRecord = { punValue }

emptyRecord = {}
