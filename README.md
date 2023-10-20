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


#### **BSE Consensus, Implementation.ipynb**:
Containing the core protocol level implementation, notably for the base useful computation, proof generation and proof validation.
Below are the key components and features:

 #### **BSE Consensus, Results.ipynb** 
 
 Empirical outputs from running the BSE Consensus protocol a few hundred times across a wide range of trader specification and market environments. 
 
 #### **BSE.py** 
 
 An adjusted version of the original Bristol Stock Exchange limit orderbook market simulation, allowing for the deterministic seeding of market sessions.




## BSE Consensus

BSE Consensus is a Python codebase for blockchain simulations and market session modeling. It allows you to create, mine, and validate blocks, and simulate market sessions with customizable parameters. Use this guide to get started with BSE Consensus:

### Getting Started

1. **Block Creation:**

   - Create a new block by initializing the `Block` class with your data, previous hash, message, user configuration, proof, and seed. You can also leave these fields empty to generate a default "Genesis Block."

2. **Mining Blocks:**

   - Use the `pre_hash` method to mine a block and generate a seed that makes the market session outcomes deterministic. Specify the mining target and optional user configuration if needed.

3. **Market Simulations:**

   - Use the `run_market_sessions` method to simulate market sessions. You can set the number of iterations and various market parameters like trader types, supply and demand curves, time intervals, and more. Market results will be logged, and you can enable options like supply and demand curve visualization.

4. **Block Validation:**

   - After market simulations, the code performs a post-hashing step to validate the market session results. It calculates the average Profit Per Second (PPS) and verifies if it meets a specified difficulty threshold.

5. **Customization and Integration:**

   - Customize the code and integrate it into your blockchain project. Modify configurations, trader types, and supply-demand curves to meet your specific requirements.

### Example Usage

Here's an example of how to use BSE Consensus:

```python
# Create a new block with custom data and configurations.
my_block = Block(data="Sample Data", user_config=my_config)

# Mine the block with a specified difficulty target.
my_block.proof_of_work(difficulty=4)

# Simulate market sessions with specific parameters.
my_block.run_market_sessions(n=10, target=4, pouw_difficulty=8, sup_dem_curve=True, posthash=True, user_config=my_config)

# Validate the results and check if the average PPS meets the difficulty threshold.
is_valid = my_block.post_hash("market_results.csv", seed=my_block.seed, pouw_difficulty=8, verifying=False)

# Output the results.
if is_valid:
    print("The market session passed the post-hashing step.")
else:
    print("The market session was rejected due to low PPS.")

# Customize the code to meet your specific project needs.

