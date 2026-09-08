module Main where

import Library (Maybe, unwrap)

forwarded first second =
  let
    extract wrapped = unwrap wrapped
  in
    { first: extract first, second: unwrap second }

signed :: Partial => Maybe Int -> Maybe Int -> { first :: Int, second :: Int }
signed = forwarded

foreign import unsafePartial :: forall a. (Partial => a) -> a

discharged :: Maybe Int -> Maybe Int -> { first :: Int, second :: Int }
discharged = unsafePartial forwarded
