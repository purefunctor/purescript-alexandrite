module Main where

parenthesized x
  | (true) = x

typed x
  | (true :: Boolean) = x
