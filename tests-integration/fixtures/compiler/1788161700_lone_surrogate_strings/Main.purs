module Main where

lead :: String
lead = "\xD800"

trail :: String
trail = "\xDFFF"

escapedPair :: String
escapedPair = "\xD800\xDC00"

scalar :: String
scalar = "\x10000"

matchesLead :: String -> Boolean
matchesLead "\xD800" = true
matchesLead _ = false
