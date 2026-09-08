module Main where

class First a

class Second a

instance First Int

instance Second Boolean

foreign import interleaved
  :: forall a. First a => (forall b. Second b => a -> b -> String)

test :: String
test = interleaved 1 true
