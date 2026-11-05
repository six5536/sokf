The sokf pack orders the work on a building block, and the recipe `recipe-plan-a-block` gives each stage:

- For a new block, the order is issue → planned design → code → contract: the issue holds the criteria and plans the files, and the criteria move into the contract in the same commit as the markers that cite them.
- For a change to an existing block, the contract stays the authority: the new criteria wait in the issue, and `Planned:` paragraphs in the contract's Inputs and Outputs describe the change until the code changes.
