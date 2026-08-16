module Main where

kinded :: ((a) :: Type) -> a
kinded value = value
