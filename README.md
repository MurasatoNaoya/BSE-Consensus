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




This repository contains the following scripts of the complete project:

#### **BSE Consensus, Implementation.ipynb**:
Containing the core protocol level implementation, notably for the base useful computation, proof generation and proof validation.
Below are the key components and features:

 #### **BSE Consensus, Results.ipynb** 
 
 Empirical outputs from running the BSE Consensus protocol a few hundred times across a wide range of trader specification and market environments. 
 
 #### **BSE.py** 
 
 An adjusted version of the original Bristol Stock Exchange limit orderbook market simulation, allowing for the deterministic seeding of market sessions.


### Usage 

1. **Block Creation:**

   - Create a new block by initialising the `Block` class with your data, previous hash, message, user configuration, proof, and seed. You can also leave these fields empty to generate a default "Genesis Block."

2. **Mining Blocks:**

   - Use the `pre_hash` method to mine a block and generate a seed that makes the market session outcomes deterministic. Specify the mining target and optional user configuration if needed.

3. **Market Simulations:**

   - Use the `run_market_sessions` method to simulate market sessions. You can set the number of iterations and various market parameters like trader types, supply and demand curves, time intervals, and more. Market results will be logged, and you can enable options like supply and demand curve visualisation.

4. **Block Validation:**

   - After market simulations, the code performs a post-hashing step to validate the market session results. It calculates the average Profit Per Second (PPS) and verifies if it meets a specified difficulty threshold.

5. **Customisation and Integration:**

   - Tailor the code and integrate it into your blockchain project. Modify configurations, trader types, and supply-demand curves to meet your specific requirements.
