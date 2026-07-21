module Main where

class Bottom a

class Bottom a <= Left a

class Bottom a <= Right a

class (Left a, Right a) <= Top a

foreign import useBottom :: forall a. Bottom a => a -> Int

foreign import useLeft :: forall a. Left a => a -> Int

foreign import useRight :: forall a. Right a => a -> Int

foreign import useTop :: forall a. Top a => a -> Int

testDiamond value =
  let
    _ = useTop value
    _ = useLeft value
    _ = useRight value
  in
    useBottom value

testShared value =
  let
    _ = useLeft value
    _ = useRight value
  in
    useBottom value
