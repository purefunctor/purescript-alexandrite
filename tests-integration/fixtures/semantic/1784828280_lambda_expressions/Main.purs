module Main where

checked :: forall a. a -> a
checked = \value -> value

inferred = \value -> value

multiple = \first second -> first
