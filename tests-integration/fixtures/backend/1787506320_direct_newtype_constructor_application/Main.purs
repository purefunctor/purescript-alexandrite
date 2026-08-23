module Main where

newtype Identity a = Identity a

direct :: Identity Int
direct = Identity 42

firstClass :: forall a. a -> Identity a
firstClass = Identity

indirect :: Identity Int
indirect = firstClass 43
