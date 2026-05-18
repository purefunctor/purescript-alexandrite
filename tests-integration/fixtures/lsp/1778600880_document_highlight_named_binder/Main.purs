module Main where

data Maybe a = Just a | Nothing

foo whole@(Just x) = whole
--  &
