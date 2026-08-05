module Main where

check outerName = do
  excludedIncompleteName <-
  continuationName <- outerName
  outerName
  let
    [patternName] = [outerName]
    equationName :: _
    equationName = outerName
  -- completion eof

sameLine outerName = do
  sameLineName <- outerName -- completion eof

nestedDo outerName = do
  outerDoName <- outerName
  do
    innerDoName <- outerName
    -- completion eof

nestedAdo outerName = ado
  outerAdoName <- outerName
  in do
    innerAdoDoName <- outerName
    -- completion eof
