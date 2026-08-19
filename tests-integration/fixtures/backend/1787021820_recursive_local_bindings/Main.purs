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
