# brute
a mnemonic brute forcing tool for nano and banano.

## What is brute?
It is a mnemonic brute forcing tool which can be used to recover an account from a partially known 24 word mnemonic phrase. This phrase must be:
- mostly known but missing entire words.
- mostly known but you don't know which exact word from a group of words should be at a given position.
- a combination of the above two. E.g. you know word #1 must be either `abandon` or `zoo`, but word #5 is completely unknown. 
- completely unknown. You don't know any of the words to the mnemonic, but you really want the nano at an address. Just kidding.

## Usage and example
To run brute, first download the latest release from the releases page or compile it yourself. Enter the directory containing brute, and run the program. 

Upon the first run, a new config file `brute_config.toml` will be created, with some default settings. To start, open this file in a text editor and paste your mnemonic into the double quotes on the line `mnemonic = ""`. A mnemonic should be in the form of 24 elements separated with a space. Each element can be one of the following:
- A word from the bip39 word list.
- A selection of words from the bip39 word list, separated by commas, that represent the possible options of the word at that position in the mnemonic.
- An X to symbolise a completely unknown word.

### Example
In this scenario, I came home after work to find my dog had got hungry whilst I was out, and eaten the first two words of my mnemonic which I had left on the table. The third word was also missing most of its letters, but I could make out it ended in a `z`. Looking in the list of the 2048 bip39 words, I could see that it could only have been `buzz`,`jazz` or `quiz`. I opened `brute_config.toml` and set my mnemonic using the appropriate syntax to reflect this:
```TOML
mnemonic = "X X buzz,jazz,quiz beach note much angry bread success carbon recall buddy fabric replace attack fruit ghost marine rural bubble spawn stem empty apart"
```
I then ran the brute program to crack my mnemonic.

## Urgent
- [ ] Error handling - standardise
- [ ] Output standardise and wait upon ending
- [ ] Implement node rpc cooldown and queuing
- [x] Config parsing
- [ ] Multi threaded cpu calculation
- [x] Split into modules and separate files
- [x] Split into more functions

## To-Do
- [ ] Allow more syntax options such as word prefix or suffix, or 'contains x' - a quick poll of the wordlist can find which apply
- [ ] GPU support for blake and sha hashes - may not be necessary if nodes cannot keep up with RPC requests...
- [ ] Get code reviewed by someone who has written more than one rust program
- [x] Split main() into sub-routines
- [x] Send node accounts_balances with a balanced number of accounts and check which contains balance (if any)
- [x] Make nano rpc requests work
