module Main where

class Parent a

class Parent a ⇐ Child a

identity ∷ ∀ a. Parent a ⇒ a → a
identity value = value

program = do
  value ← action
  pure value
