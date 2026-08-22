module Main where

foreign import same :: forall value. value -> value -> Boolean

foreign import observe :: forall value. String -> value -> value

foreign import readTrace :: Boolean -> Array String

applicationRecursive :: Boolean -> { result :: Int, trace :: Array String }
applicationRecursive token =
  let
    first :: Boolean -> Int
    first = observe "first" \condition -> if condition then second false else 1

    second :: Boolean -> Int
    second = observe "second" \condition -> if condition then first false else 2
  in
    { result: first true, trace: readTrace token }

recordRecursive :: Boolean
recordRecursive =
  let
    first :: { value :: Boolean -> Int }
    first = { value: second }

    second :: Boolean -> Int
    second condition = if condition then first.value false else 20
  in
    same first.value second

caseRecursive :: Boolean -> Int
caseRecursive condition = go true
  where
  go :: Boolean -> Int
  go = case condition of
    true -> \current -> if current then go false else 30
    false -> \_ -> 31

letRecursive :: Boolean -> Int
letRecursive condition = go condition
  where
  go :: Boolean -> Int
  go =
    let
      wrapped = \current -> if current then go false else 40
    in
      wrapped

strictCycle :: Boolean -> Int
strictCycle _ = value
  where
  value :: Int
  value = wrap value

wrap :: forall value. value -> value
wrap value = value
