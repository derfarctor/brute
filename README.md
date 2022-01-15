# brute
a mnemonic brute forcing tool for nano and banano.

## What is brute?
This is my first project in rust. It is a mnemonic brute forcing tool which can be used to recover accounts to which the mnemonic is:
- mostly known but missing entire words
- mostly known but you don't know which exact word from a select few should be at a given position
- completely unknown. You don't know any of the words to the mnemonic, but you really want the nano at the address. Just kidding.

## Usage and example
To run brute, first download the latest release from the releases page or compile it yourself. Open a cmd window or terminal, and navigate to the directory that brute.exe is installed to.
```
brute mnemonic
```
is the basic usage, whereby mnemonic refers to the known part of the mnemonic, with unknowns following these rules:
- replace fully unknown words with X
- for unknown words which must come from a known group of words, replace with this group joined by commas
### Example
You find the paper with your mnemonic phrase ripped apart with only **21 out of 24** words showing (*The first two words are completely missing and only the final letter of the third remains, after your dog got hungry whilst you were at work*). You can make out that the third word ended with a 'z', and after checking the list of bip39 words, conclude it must be one of quiz, jazz and buzz. 
```
brute X X quiz,jazz,buzz modify offer van door kangaroo hope drop ice ghost vendor bread electric brief erupt virus lend course link soldier flight window
```

## To-Do
- [ ] Make nano rpc requests work
- [ ] Multi-threaded cpu mnemonic to addr generation
- [ ] GPU support for blake and sha hashes - may not be necessary if nodes cannot keep up with RPC requests...
- [ ] Error handling upon mnemonic input, as well as in node RPC requests
- [ ] Proper config structure rework - config and log file
- [ ] Allow more syntax options such as word prefix or suffix, or 'contains x' - a quick poll of the wordlist can find which apply
- [x] Split main() into sub-routines
- [x] Send node accounts_balances with a balanced number of accounts and check which contains balance (if any)
- [ ] Get code reviewed by someone who has written more than one rust program
