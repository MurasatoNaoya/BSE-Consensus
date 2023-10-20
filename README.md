# BSE Consensus README


The consensus primitive known as Proof of Work (PoW) has long had an integral role in securing
the Bitcoin network and other related protocols. Despite its effectiveness, by the nature of this PoW
mechanism, an enormous amount of purposefully arbitrary computation must be done by a miner to verify
their legitimacy over time, earn the right to publish a block of transactions and then reap its associated
rewards. Over concerns regarding the adverse effects computation of such scale has on global energy
supplies and the wider environment in the form of negative externalities, and more blockchain specific
considerations like protocol efficiency, methods of reducing such arbitrary computation while redirecting it
to more subjectively societally-beneficial means in the form of Proof of Useful Work (PoUW) has long been
a hopeful, but largely fruitless area of debate. While theoretical proposals of PoUW have been discussed
in the past, few; if any actual implementations that promote the efficiency and generic ’usefulness’ of
computation beyond more obscure, narrow scientific applications, while maintaining network security
currently exist.


Drawing from other very recent and innovative academic literature like IOHK’s Ofelimos
that aim to realise this notion of PoUW, this project builds upon existing conceptions and presents a firstof-its-kind proof of concept implementation of a PoUW consensus mechanism called BSE Consensus.
Involving the training of trading algorithms within a simulated trading environment as the basis of its
necessarily difficult, but relatively useful computation, specifically integrating the limit order book trading
environment simulator the Bristol Stock Exchange (BSE). Analysis of the protocol relating to its security
is also conducted to ensure the integrity of the underlying consensus mechanism, and further investigation
into of the protocol’s efficiency and overall usefulness is conducted too, with the ultimate aim of better
understanding the viability using BSE as a foundation for a protocol based puzzle schemes and the future
of PoUW consensus protocols more generally.




This repository containing the following scripts of the complete project - 


### **BSE Consensus, Implementation.ipynb**:
Containing the core protocol level implementation, notably for the base useful computation, proof generation and proof validation.
Below are the key components and features:

#### **Block Class:** 

Represents individual blocks in the blockchain.
Contains methods for calculating hashes, performing proof of work, and managing market session details.


#### **Blockchain Class**:

Manages the blockchain and its consensus process.
Handles block validation and mining using BSE or standard proof of work.

#### **BSE Integration**:

Integrates BSE functionality to simulate market sessions for block creation.
Allows users to submit job requests for custom market sessions.

 ### **BSE Consensus, Results.ipynb** 
 
 Empirical outputs from running the BSE Consensus protocol a few hundred times across a wide range of trader specification and market environments. 
 
 ### **BSE.py** 
 
 An adjusted version of the original Bristol Stock Exchange limit orderbook market simulation, allowing for the deterministic seeding of market sessions.
