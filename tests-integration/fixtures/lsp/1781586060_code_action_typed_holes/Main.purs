module Main where

termHole :: Int -> Int
termHole argument =
  let
    localInt :: Int
    localInt = argument

    localString :: String
    localString = "not relevant"
  in
    ?term
--  .

type TypeHole a = ?type :: Type
--                .

data Choice = Yes | No

constructorHole :: Choice
constructorHole = ?constructor
--                .
