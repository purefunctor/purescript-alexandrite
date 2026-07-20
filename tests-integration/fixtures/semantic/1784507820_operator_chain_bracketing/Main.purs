module Main where

foreign import add :: Int -> Int -> Int
foreign import multiply :: Int -> Int -> Int
foreign import append :: Int -> Int -> Int

infixl 6 add as +
infixl 7 multiply as *
infixr 5 append as <+>

precedence = 1 + 2 * 3 + 4

rightAssociative = 1 <+> 2 <+> 3

prefix = (+) 1 2
