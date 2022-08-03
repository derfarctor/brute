# brute
a mnemonic brute forcing tool for nano and banano.

## What is brute?
It is a mnemonic brute forcing tool which can be used to recover an account from a partially known 24 word mnemonic phrase. This phrase must be:
- mostly known but missing entire words.
- mostly known but you don't know which exact word from a group of words should be at a given position.
- a combination of the above two. E.g. you know word #1 must be either `abandon` or `zoo`, but word #5 is completely unknown. 
- completely unknown. You don't know any of the words to the mnemonic, but you really want the nano at an address. Just kidding.

## The three flavours of brute
The brute tool can be ran in three different modes:
1. **Address mode** - This is a highest performance version of brute which assumes you know the address (or possible addresses) of the account you wish to try and recover. 

Flavours 2 and 3 assume that you have not managed to retain the address to the account you are trying to brute force, and hence each address generated must be checked to see if it has a balance which is much slower. This can be done in two ways.

2. **Ledger mode** - This is the high performance, offline version of brute which requires a `data.ldb` file. This is the ledger database of the network you wish to recover an account from. The database snapshot must have been taken after the account you wish to recover was first opened.

3. **Node mode** - This much slower version of brute does not require the downloading of the ledger database. Instead, brute communicates with a node via rpc. Due to node limitations, this is restricted to single thread cpu calculations.

## Usage and example
To run brute, first download the latest release from the releases page or compile it yourself. Enter the directory containing brute. If on windows, double click on the supplied `run_brute.bat`, otherwise run the brute program.

Upon the first run, a new config file `brute_config.toml` will be created, with some default settings. To start, open `brute_config.toml` in a text editor and paste your mnemonic into the double quotes on the line `mnemonic = ""`. A mnemonic should be in the form of 24 elements separated with a space. Each element can be one of the following:
- A word from the bip39 word list.
- A selection of words from the bip39 word list, separated by commas, that represent the possible options of the word at that position in the mnemonic.
- An X to symbolise a completely unknown word.

If you wish to run brute in Ledger or Node mode, you must now configure the settings. These are self explanatory but a detailed overview of each setting is available at the bottom of this readme under the `Options in brute_config.toml` title.

Once your mnemonic has been added to the config file, run brute again - if on windows, via `run_brute.bat` like before.

### Example
In this scenario, I came home after work to find my dog had got hungry whilst I was out, and eaten the first two words of my mnemonic which I had left on the table. The third word was also missing most of its letters, but I could make out it ended in a `z`. Looking in the list of the 2048 bip39 words, I could see that it could only have been `buzz`,`jazz` or `quiz`. I opened `brute_config.toml` and set my mnemonic using the appropriate syntax to reflect this:
```TOML
mnemonic = "X X buzz,jazz,quiz beach note much angry bread success carbon recall buddy fabric replace attack fruit ghost marine rural bubble spawn stem empty apart"
```
I then ran the brute program to crack my mnemonic.

## Options in brute_config.toml
### General settings:
- **mode** - This will tell brute the mode to run in. 1 for address mode, 2 for ledger mode, and 3 for node mode.
- **address_prefix** - This will tell brute the type of address you are looking for. If looking for a nano account, this should remain 'nano_'. If looking for a banano account, this should be set to 'ban_'. If you are using brute in Ledger mode, this is purely cosmetic since the tool checks the ledger for public keys, not addresses.

- **stop_at_first** - This option can be toggled true/false. Tells brute whether or not to stop searching for opened accounts, once the first opened account has been found.

- **stats_logging** - This option can be toggled true/false. Tells brute whether or not to print statistics each second - % remaining and mnemonics per second.

- **multithreaded** - This option can be toggled true/false. Tells brute whether or not to effeciently use all cores on your computer for computations. Settings this to true may stress your CPU to a greater degree.

### Ledger settings:
- **ledger_path** - If you wish to use the Ledger mode, supply the full path to your `data.ldb` file here.


### Node settings:
- **node_url** - The Node RPC endpoint that you wish to query for account balance checking. This is important, and if possible you should set this to your own node. **brute** makes a lot of requests, and stressing public nodes is not preferable.

- **batch_size** - This is the number of accounts per 'accounts_balances' request sent to the node. Too little, and the frequency of requests becomes too large. Too much, and the node will likely reject your request. Change with caution. For RPC Proxy enabled nodes, this will need to be lowered from 10,000 to 1,000 in most cases.

- **request_cooldown** - Not yet implemented.
