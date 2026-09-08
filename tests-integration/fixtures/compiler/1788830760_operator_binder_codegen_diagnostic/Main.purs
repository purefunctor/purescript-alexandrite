module Main where

newtype N = N Int

infix 5 N as :+

test (a :+ b) = 1
