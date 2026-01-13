class BalanceTracker {
    constructor(){
        this.balances = new Map();
    }

    updateBalance(blockchain) {
        this.balances.clear();

        // xử lý coin transaction trong các block
        blockchain.forEach(block => {
            if(block.coi)
            
        });
    }
}