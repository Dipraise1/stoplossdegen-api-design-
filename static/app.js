// API URL - dynamically determine the base URL from the browser's location
const API_URL = window.location.origin;

// DOM elements
const generateWalletBtn = document.getElementById('generate-wallet-btn');
const importWalletBtn = document.getElementById('import-wallet-submit');
const refreshBalancesBtn = document.getElementById('refresh-balances-btn');
const refreshPricesBtn = document.getElementById('refresh-prices-btn');
const refreshOrdersBtn = document.getElementById('refresh-orders-btn');
const limitOrderForm = document.getElementById('limit-order-form');
const sourceTokenSelect = document.getElementById('source-token');
const targetTokenSelect = document.getElementById('target-token');
const walletPubkeyDisplay = document.getElementById('wallet-pubkey');
const tokenBalancesDiv = document.getElementById('token-balances');
const tokenPricesDiv = document.getElementById('token-prices');
const ordersTableBody = document.getElementById('orders-table-body');
const noWalletAlert = document.getElementById('no-wallet-alert');
const walletDetails = document.getElementById('wallet-details');
const orderTypeSelect = document.getElementById('order-type');
const amountLabel = document.getElementById('amount-label');
const priceTargetLabel = document.getElementById('price-target-label');

// Known token data
const knownTokens = [
    { symbol: 'SOL', mint: 'So11111111111111111111111111111111111111112' },
    { symbol: 'USDC', mint: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v' },
    { symbol: 'BONK', mint: 'DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263' },
    { symbol: 'GMT', mint: '7i5KKsX2weiTkry7jA4ZwSuXGhs5eJBEjY8vVxR4pfRx' }
];

// Bootstrap modals
const loadingModal = new bootstrap.Modal(document.getElementById('loadingModal'));
const walletGeneratedModal = new bootstrap.Modal(document.getElementById('walletGeneratedModal'));
const importWalletModal = new bootstrap.Modal(document.getElementById('importWalletModal'));

// Event listeners
document.addEventListener('DOMContentLoaded', initApp);
generateWalletBtn.addEventListener('click', generateWallet);
importWalletBtn.addEventListener('click', importWallet);
refreshBalancesBtn.addEventListener('click', fetchBalances);
refreshPricesBtn.addEventListener('click', fetchPrices);
refreshOrdersBtn.addEventListener('click', fetchOrders);
limitOrderForm.addEventListener('submit', (e) => {
    e.preventDefault();
    createLimitOrder(new FormData(limitOrderForm));
});

// Update labels based on order type
orderTypeSelect.addEventListener('change', () => {
    const orderType = orderTypeSelect.value;
    
    if (orderType === 'buy') {
        amountLabel.textContent = 'Amount to Spend';
        priceTargetLabel.textContent = 'Buy when price is at or below';
    } else if (orderType === 'sell') {
        amountLabel.textContent = 'Amount to Sell';
        priceTargetLabel.textContent = 'Sell when price is at or above';
    } else if (orderType === 'stop_loss') {
        amountLabel.textContent = 'Amount to Sell';
        priceTargetLabel.textContent = 'Sell when price drops to';
    }
});

// Initialize the app
async function initApp() {
    // Check if we have a wallet in localStorage
    const storedPubkey = localStorage.getItem('walletPubkey');
    if (storedPubkey) {
        showWalletConnected(storedPubkey);
        await Promise.all([
            fetchBalances(),
            fetchPrices(),
            fetchOrders()
        ]);
    } else {
        // Just fetch prices if no wallet
        await fetchPrices();
    }

    // Populate token selects
    populateTokenSelects();
    
    // Set initial form labels
    if (orderTypeSelect) {
        // Trigger the change event to set initial label text
        const event = new Event('change');
        orderTypeSelect.dispatchEvent(event);
    }
}

// Generate a new wallet
async function generateWallet() {
    showLoading('Generating new wallet...');
    
    try {
        const response = await fetch(`${API_URL}/generate_wallet`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            }
        });
        
        const data = await response.json();
        
        if (data.success) {
            const { pubkey, mnemonic } = data.data;
            
            // Store the pubkey in localStorage
            localStorage.setItem('walletPubkey', pubkey);
            
            // Show the generated wallet info
            document.getElementById('generated-pubkey').value = pubkey;
            document.getElementById('generated-mnemonic').value = mnemonic;
            
            // Update UI to show wallet is connected
            showWalletConnected(pubkey);
            
            // Show the generated wallet modal with the mnemonic
            hideLoading();
            walletGeneratedModal.show();
            
            // Fetch balances and orders
            await Promise.all([
                fetchBalances(),
                fetchOrders()
            ]);
        } else {
            showError('Failed to generate wallet: ' + data.error);
        }
    } catch (error) {
        showError('Error generating wallet: ' + error.message);
    }
    
    hideLoading();
}

// Import an existing wallet
async function importWallet() {
    const activeTab = document.querySelector('#importTabs .nav-link.active').id;
    let requestData = {};
    
    if (activeTab === 'private-key-tab') {
        const privateKey = document.getElementById('private-key-input').value.trim();
        if (!privateKey) {
            alert('Please enter a private key');
            return;
        }
        requestData.private_key = privateKey;
    } else {
        const mnemonic = document.getElementById('mnemonic-input').value.trim();
        if (!mnemonic) {
            alert('Please enter a mnemonic phrase');
            return;
        }
        requestData.mnemonic = mnemonic;
    }
    
    showLoading('Importing wallet...');
    
    try {
        const response = await fetch(`${API_URL}/import_wallet`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify(requestData)
        });
        
        const data = await response.json();
        
        if (data.success) {
            const { pubkey } = data.data;
            
            // Store the pubkey in localStorage
            localStorage.setItem('walletPubkey', pubkey);
            
            // Update UI to show wallet is connected
            showWalletConnected(pubkey);
            
            // Clear input fields
            document.getElementById('private-key-input').value = '';
            document.getElementById('mnemonic-input').value = '';
            
            // Close the import modal
            importWalletModal.hide();
            
            // Fetch balances and orders
            await Promise.all([
                fetchBalances(),
                fetchOrders()
            ]);
        } else {
            showError('Failed to import wallet: ' + data.error);
        }
    } catch (error) {
        showError('Error importing wallet: ' + error.message);
    }
    
    hideLoading();
}

// Fetch token balances
async function fetchBalances() {
    if (!localStorage.getItem('walletPubkey')) {
        tokenBalancesDiv.innerHTML = '<p>Connect a wallet to view balances</p>';
        return;
    }
    
    tokenBalancesDiv.innerHTML = '<p>Loading balances...</p>';
    
    try {
        const response = await fetch(`${API_URL}/get_balances`);
        const data = await response.json();
        
        if (data.success) {
            const balances = data.data;
            
            if (balances.length === 0) {
                tokenBalancesDiv.innerHTML = '<p>No token balances found</p>';
                return;
            }
            
            // Render the balances
            let html = '';
            balances.forEach(balance => {
                html += `
                    <div class="token-balance">
                        <div class="token-symbol">${balance.symbol}</div>
                        <div class="token-amount">${parseFloat(balance.amount).toFixed(6)}</div>
                    </div>
                `;
            });
            
            tokenBalancesDiv.innerHTML = html;
        } else {
            tokenBalancesDiv.innerHTML = `<p class="text-danger">Error: ${data.error}</p>`;
        }
    } catch (error) {
        tokenBalancesDiv.innerHTML = `<p class="text-danger">Error: ${error.message}</p>`;
    }
}

// Fetch token prices
async function fetchPrices() {
    tokenPricesDiv.innerHTML = '<p>Loading prices...</p>';
    
    try {
        const response = await fetch(`${API_URL}/get_prices`);
        const data = await response.json();
        
        if (data.success) {
            const prices = data.data;
            
            if (prices.length === 0) {
                tokenPricesDiv.innerHTML = '<p>No price data available</p>';
                return;
            }
            
            // Render the prices
            let html = '';
            prices.forEach(price => {
                html += `
                    <div class="token-price">
                        <div class="token-symbol">${price.symbol}</div>
                        <div class="token-amount">$${parseFloat(price.price_usd).toFixed(6)}</div>
                    </div>
                `;
            });
            
            tokenPricesDiv.innerHTML = html;
        } else {
            tokenPricesDiv.innerHTML = `<p class="text-danger">Error: ${data.error}</p>`;
        }
    } catch (error) {
        tokenPricesDiv.innerHTML = `<p class="text-danger">Error: ${error.message}</p>`;
    }
}

// Fetch active orders
async function fetchOrders() {
    if (!localStorage.getItem('walletPubkey')) {
        ordersTableBody.innerHTML = '<tr><td colspan="8" class="text-center">No wallet connected</td></tr>';
        return;
    }
    
    try {
        const response = await fetch(`${API_URL}/list_limit_orders`);
        const data = await response.json();
        
        if (data.success) {
            const orders = data.data;
            
            if (orders.length === 0) {
                ordersTableBody.innerHTML = '<tr><td colspan="8" class="text-center">No active orders</td></tr>';
                return;
            }
            
            // Render the orders
            let html = '';
            orders.forEach(order => {
                const sourceSymbol = getTokenSymbol(order.source_token);
                const targetSymbol = getTokenSymbol(order.target_token);
                
                html += `
                    <tr>
                        <td>${order.id.slice(0, 8)}...</td>
                        <td>${order.order_type}</td>
                        <td>${sourceSymbol}</td>
                        <td>${targetSymbol}</td>
                        <td>${parseFloat(order.amount).toFixed(6)}</td>
                        <td>$${parseFloat(order.price_target).toFixed(6)}</td>
                        <td class="status-${order.status.toLowerCase()}">${order.status}</td>
                        <td>
                            <button 
                                class="btn btn-sm btn-danger" 
                                onclick="cancelOrder('${order.id}')"
                                ${order.status !== 'ACTIVE' ? 'disabled' : ''}
                            >
                                Cancel
                            </button>
                        </td>
                    </tr>
                `;
            });
            
            ordersTableBody.innerHTML = html;
        } else {
            ordersTableBody.innerHTML = `<tr><td colspan="8" class="text-center text-danger">Error: ${data.error}</td></tr>`;
        }
    } catch (error) {
        ordersTableBody.innerHTML = `<tr><td colspan="8" class="text-center text-danger">Error: ${error.message}</td></tr>`;
    }
}

// Create a limit order
async function createLimitOrder(formData) {
    if (!localStorage.getItem('walletPubkey')) {
        showError('Please connect a wallet first');
        return;
    }
    
    const orderType = document.getElementById('order-type').value;
    const sourceToken = document.getElementById('source-token').value;
    const targetToken = document.getElementById('target-token').value;
    const amount = parseFloat(document.getElementById('amount').value);
    const priceTarget = parseFloat(document.getElementById('price-target').value);
    
    if (sourceToken === targetToken) {
        showError('Source and target tokens cannot be the same');
        return;
    }
    
    if (amount <= 0) {
        showError('Amount must be greater than zero');
        return;
    }
    
    if (priceTarget <= 0) {
        showError('Price target must be greater than zero');
        return;
    }
    
    showLoading('Creating limit order...');
    
    try {
        const requestData = {
            order_type: orderType,
            source_token: sourceToken,
            target_token: targetToken,
            amount: amount,
            price_target: priceTarget
        };
        
        const response = await fetch(`${API_URL}/set_limit_order`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify(requestData)
        });
        
        const data = await response.json();
        
        if (data.success) {
            // Clear the form
            document.getElementById('amount').value = '';
            document.getElementById('price-target').value = '';
            
            // Refresh the orders list
            await fetchOrders();
            
            // Show success message
            alert('Limit order created successfully');
        } else {
            showError('Failed to create limit order: ' + data.error);
        }
    } catch (error) {
        showError('Error creating limit order: ' + error.message);
    }
    
    hideLoading();
}

// Cancel an order
async function cancelOrder(orderId) {
    if (!confirm('Are you sure you want to cancel this order?')) {
        return;
    }
    
    showLoading('Cancelling order...');
    
    try {
        const response = await fetch(`${API_URL}/cancel_limit_order`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({ order_id: orderId })
        });
        
        const data = await response.json();
        
        if (data.success) {
            // Refresh the orders list
            await fetchOrders();
        } else {
            showError('Failed to cancel order: ' + data.error);
        }
    } catch (error) {
        showError('Error cancelling order: ' + error.message);
    }
    
    hideLoading();
}

// Update UI to show connected wallet
function showWalletConnected(pubkey) {
    noWalletAlert.style.display = 'none';
    walletDetails.style.display = 'block';
    walletPubkeyDisplay.textContent = pubkey;
}

// Populate token selection dropdowns
function populateTokenSelects() {
    // Clear existing options
    sourceTokenSelect.innerHTML = '';
    targetTokenSelect.innerHTML = '';
    
    // Add token options
    knownTokens.forEach(token => {
        const sourceOption = document.createElement('option');
        sourceOption.value = token.mint;
        sourceOption.textContent = token.symbol;
        sourceTokenSelect.appendChild(sourceOption);
        
        const targetOption = document.createElement('option');
        targetOption.value = token.mint;
        targetOption.textContent = token.symbol;
        targetTokenSelect.appendChild(targetOption);
    });
    
    // Set different default selections for source and target
    if (sourceTokenSelect.options.length > 0) {
        sourceTokenSelect.selectedIndex = 0;
    }
    
    if (targetTokenSelect.options.length > 1) {
        targetTokenSelect.selectedIndex = 1;
    }
}

// Get token symbol from mint address
function getTokenSymbol(mintAddress) {
    const token = knownTokens.find(t => t.mint === mintAddress);
    return token ? token.symbol : mintAddress.slice(0, 6) + '...';
}

// Show loading modal
function showLoading(message = 'Processing request...') {
    document.getElementById('loading-message').textContent = message;
    loadingModal.show();
}

// Hide loading modal
function hideLoading() {
    loadingModal.hide();
}

// Show error message
function showError(message) {
    hideLoading();
    alert(message);
}

// Expose the cancel order function to the global scope
window.cancelOrder = cancelOrder; 