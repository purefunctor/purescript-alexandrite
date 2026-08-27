module Main where

foreign import addInt :: Int -> Int -> Int

data Tree = Leaf Int | Branch Tree Tree

data TreeStack = Empty | Push Tree TreeStack

sumTree :: Tree -> Int
sumTree tree = walkTree tree Empty 0

walkTree :: Tree -> TreeStack -> Int -> Int
walkTree tree stack accumulator = case tree of
  Leaf value -> case stack of
    Empty -> addInt accumulator value
    Push next rest -> walkTree next rest (addInt accumulator value)
  Branch left right -> walkTree left (Push right stack) accumulator
