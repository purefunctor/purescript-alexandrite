module Main where

invalidExpression :: String
invalidExpression = "\q"

invalidBinder :: String -> Boolean
invalidBinder "\q" = true
invalidBinder _ = false

type InvalidType = "\q"
