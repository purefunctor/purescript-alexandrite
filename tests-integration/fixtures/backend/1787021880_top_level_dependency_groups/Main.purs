module Main where

forward :: Int
forward = later

later :: Int
later = 42

first :: Boolean -> Int
first true = 1
first false = second true

second :: Boolean -> Int
second true = 2
second false = first true
