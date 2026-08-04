module Main where

class Renamed a where
  renamedMember :: a

class Original a where
--    /
  originalMember :: a
