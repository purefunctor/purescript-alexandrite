module Main where

check outerName = do
  let { nested: { patternNestedName }, patternPunName } = outerName
  
-- completion eof
