module HiddenLib where

class Hidden f where
  hidden :: forall a. f a -> a
