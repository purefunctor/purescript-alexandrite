module Main where

missingScrutinee = case of
  _ -> true

mismatchedBinders first second = case first, second of
  _ -> true

missingResult value = case value of
  _ ->

missingGuardResult value = case value of
  _ | true ->

missingBranchBody value = case value of
  _
