# Blabber

This generates random strings from a from a given context-free grammar.

## BNF Notation

Blabber accepts grammar descriptions in Brackus-Naur Form. Tokens are seperated by whitespace, and they can contain any non-whitespace unicode characters. Nonterminals are denoted by double quotes. As an example, here is the US post address format, written in Blabber's BNF dialect.
```
postal.address = name.part street.address zip.part
name.part = personal.part last.name opt.suffix.part "\n" | personal.part name.part
personal.part = first.name | initial "."
street.address = house.num street.name opt.apt.num "\n"
zip.part = town.name "," state.code zip.code "\n"
opt.suffix.part = "Sr." | "Jr." | roman.numeral | ""
opt.apt.num = "Apt" apt.num | ""
```