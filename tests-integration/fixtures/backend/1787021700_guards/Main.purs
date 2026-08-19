module Main where

data Choice a = Empty | One a

booleanGuard :: Boolean -> Int
booleanGuard value
  | value = 1
  | true = 0

patternGuard :: Choice Int -> Int
patternGuard choice
  | One value <- choice = value
  | true = 0

caseBooleanGuard :: Boolean -> Int
caseBooleanGuard value = case value of
  _ | false -> 1
  _ -> 2

casePatternGuard :: Choice Int -> Int
casePatternGuard choice = case choice of
  _ | One value <- choice -> value
  _ -> 0

nestedCaseGuard :: Boolean -> Int
nestedCaseGuard value = case value of
  true | true -> case value of
    _ | false -> 1
    _ -> 2
  _ -> 3
