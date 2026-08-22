module Main where

mutual :: Boolean -> Int
mutual condition = first condition
  where
  first :: Boolean -> Int
  first true = 1
  first false = second true

  second :: Boolean -> Int
  second true = 2
  second false = first true

capturedMutual :: Int -> Boolean -> Int
capturedMutual captured condition = first condition
  where
  first :: Boolean -> Int
  first true = captured
  first false = second true

  second :: Boolean -> Int
  second true = captured
  second false = first true

nestedRecursive :: Boolean -> Int
nestedRecursive condition = go condition
  where
  go :: Boolean -> Int
  go true =
    let
      nested = go false
    in
      nested
  go false = 0
