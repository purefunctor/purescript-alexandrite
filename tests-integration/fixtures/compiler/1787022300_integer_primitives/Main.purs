module Main where

foreign import addInt :: Int -> Int -> Int
foreign import multiplyInt :: Int -> Int -> Int

constantAdd :: Int
constantAdd = addInt 20 22

constantMultiply :: Int
constantMultiply = multiplyInt 6 7

overflowAdd :: Int
overflowAdd = addInt 2147483647 1

unknownAdd :: Int -> Int -> Int
unknownAdd = addInt
