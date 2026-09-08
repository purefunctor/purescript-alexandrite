module Main where

class Bottom a

class Bottom a <= Middle a

class Middle a <= Top a

foreign import useBottom :: forall a. Bottom a => a -> Int

foreign import useMiddle :: forall a. Middle a => a -> Int

foreign import useTop :: forall a. Top a => a -> Int

test value =
  let
    _ = useTop value
    _ = useMiddle value
  in
    useBottom value
