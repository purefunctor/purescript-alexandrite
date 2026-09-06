module Main where

data Maybe a = Just a | Nothing

local :: Partial => Maybe Int -> Int
local choice =
  let
    unwrap wrapped =
      let
        Just value = wrapped
      in
        value
  in
    unwrap choice
