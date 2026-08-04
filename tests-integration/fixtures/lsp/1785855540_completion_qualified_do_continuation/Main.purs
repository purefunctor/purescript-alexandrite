module Main where

import Effect as Qualified

check outerName = Qualified.do
  qualifiedBoundName <- outerName
  
-- completion eof
