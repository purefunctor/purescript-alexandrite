module Main where

import Control.Bind (bind)
import Effect (Effect)

foreign import constructEffect :: forall a. String -> a -> Effect a

branched :: Boolean -> String -> Effect String
branched choose seed = do
  value <- constructEffect "branch-action" seed
  if choose then
    constructEffect "branch-then" value
  else
    constructEffect "branch-else" value

patternLet :: String -> Effect String
patternLet seed = do
  value <- constructEffect "pattern-action" seed
  let
    { selected } = { selected: value }
  constructEffect "pattern-result" selected
