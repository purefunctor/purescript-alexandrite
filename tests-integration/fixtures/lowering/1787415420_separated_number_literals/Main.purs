module Main where

separatedNumber = 4_294_967_295.0

separatedNumberParts = 1_2.3_4e+0_2

separatedExponent = 1_2e0_2

separatedUpperExponent = 1_3E0_2

separatedNegativeExponent = 1_400e-0_2

matchesSeparatedNumber 1_0.0_5e+0_2 = true
matchesSeparatedNumber 1_2e0_2 = true
matchesSeparatedNumber 1_3E0_2 = true
matchesSeparatedNumber 1_400e-0_2 = true
matchesSeparatedNumber (-1_5.0) = true
matchesSeparatedNumber _ = false
