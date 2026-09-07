module Main where

import Prim.TypeError (class Fail, Text)

guardedFailure :: Fail (Text "failure was demanded") => Int
guardedFailure = 0

test = if true then test else guardedFailure
