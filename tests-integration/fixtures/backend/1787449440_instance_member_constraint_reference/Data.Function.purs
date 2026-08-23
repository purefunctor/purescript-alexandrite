module Data.Function (on) where

on :: forall a b c. (b -> b -> c) -> (a -> b) -> a -> a -> c
on operation projection left right = operation (projection left) (projection right)
