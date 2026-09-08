module Main where

foreign import collect :: Int -> Int -> Int -> Array Int

foreign import failAt :: String -> Boolean -> Int -> Int

foreign import observe :: String -> Int -> Int

foreign import observedRecord :: { value :: Int }

ordered :: Boolean -> Boolean -> Array Int
ordered branch shouldThrow =
  let
    choose value =
      if branch then observe "branch-true" value
      else observe "branch-false" value
  in
    collect
      (observe "before" observedRecord.value)
      (choose (failAt "middle" shouldThrow 2))
      (observe "after" 3)

reused :: Array Int
reused =
  let value = observe "reused" 4
  in [ value, value ]
