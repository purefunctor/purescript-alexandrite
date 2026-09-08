module Main where

identity value = value

empty :: Array Int
empty = []

values :: Array Int
values = [1, 2]

nested :: Array (Array Int)
nested = [[1], [2, 3]]

inferredFunctions = [identity]

checkedFunctions :: Array (Int -> Int)
checkedFunctions = [identity]

checkedIdentity :: Array Int -> Array Int
checkedIdentity = identity

consume :: Array Int -> Array Int
consume value = value

application :: Array Int
application = consume [1, 2]
