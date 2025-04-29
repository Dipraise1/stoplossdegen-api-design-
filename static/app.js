// API URL - dynamically determine the base URL from the browser's location
const API_URL = window.location.origin;

// DOM elements
const generateWalletBtn = document.getElementById('generate-wallet-btn');
const importWalletBtn = document.getElementById('import-wallet-btn');
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
    const orderTypeSelect = document.getElementById('order-type');
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
        const response = await axios.post(`${API_URL}/generate_wallet`);
        
        if (response.data.success) {
            const { pubkey, mnemonic } = response.data.data;
            
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
            showError('Failed to generate wallet: ' + response.data.error);
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
        const response = await axios.post(`${API_URL}/import_wallet`, requestData);
        
        if (response.data.success) {
            const { pubkey } = response.data.data;
            
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
            showError('Failed to import wallet: ' + response.data.error);
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
        const response = await axios.get(`${API_URL}/get_balances`);
        
        if (response.data.success) {
            const balances = response.data.data;
            
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
                        <div class="token-amount">${balance.amount.toFixed(6)}</div>
                    </div>
                `;
            });
            
            tokenBalancesDiv.innerHTML = html;
        } else {
            tokenBalancesDiv.innerHTML = `<p class="text-danger">Error: ${response.data.error}</p>`;
        }
    } catch (error) {
        tokenBalancesDiv.innerHTML = `<p class="text-danger">Error: ${error.message}</p>`;
    }
}

// Fetch token prices
async function fetchPrices() {
    tokenPricesDiv.innerHTML = '<p>Loading prices...</p>';
    
    try {
        const response = await axios.get(`${API_URL}/get_prices`);
        
        if (response.data.success) {
            const prices = response.data.data;
            
            if (prices.length === 0) {
                tokenPricesDiv.innerHTML = '<p>No token prices found</p>';
                return;
            }
            
            // Render the prices
            let html = '';
            prices.forEach(price => {
                html += `
                    <div class="token-price">
                        <div class="token-symbol">${price.symbol}</div>
                        <div class="token-amount">$${price.price_usd.toFixed(4)}</div>
                    </div>
                `;
            });
            
            tokenPricesDiv.innerHTML = html;
        } else {
            tokenPricesDiv.innerHTML = `<p class="text-danger">Error: ${response.data.error}</p>`;
        }
    } catch (error) {
        tokenPricesDiv.innerHTML = `<p class="text-danger">Error: ${error.message}</p>`;
    }
}

// Fetch limit orders
async function fetchOrders() {
    if (!localStorage.getItem('walletPubkey')) {
        ordersTableBody.innerHTML = '<tr><td colspan="9" class="text-center">Connect a wallet to view orders</td></tr>';
        return;
    }
    
    try {
        const response = await axios.get(`${API_URL}/list_limit_orders`);
        
        if (response.data.success) {
            const orders = response.data.data;
            
            if (orders.length === 0) {
                ordersTableBody.innerHTML = '<tr><td colspan="9" class="text-center">No orders found</td></tr>';
                return;
            }
            
            // Render the orders
            let html = '';
            orders.forEach(order => {
                // Get token symbols
                const sourceSymbol = getTokenSymbol(order.source_token);
                const targetSymbol = getTokenSymbol(order.target_token);
                
                // Format date
                const createdDate = new Date(order.created_at).toLocaleString();
                
                // Determine status class
                const statusClass = `status-${order.status.toLowerCase()}`;
                
                html += `
                    <tr>
                        <td><span class="pubkey-truncate">${order.id.substring(0, 8)}...</span></td>
                        <td>${order.order_type}</td>
                        <td>${sourceSymbol}</td>
                        <td>${targetSymbol}</td>
                        <td>${order.amount.toFixed(6)}</td>
                        <td>$${order.price_target.toFixed(4)}</td>
                        <td><span class="${statusClass}">${order.status}</span></td>
                        <td>${createdDate}</td>
                        <td>
                            ${order.status === 'Active' ? `
                                <button class="btn btn-sm btn-danger btn-action" 
                                        onclick="cancelOrder('${order.id}')">Cancel</button>
                            ` : ''}
                        </td>
                    </tr>
                `;
            });
            
            ordersTableBody.innerHTML = html;
        } else {
            ordersTableBody.innerHTML = `<tr><td colspan="9" class="text-danger">Error: ${response.data.error}</td></tr>`;
        }
    } catch (error) {
        ordersTableBody.innerHTML = `<tr><td colspan="9" class="text-danger">Error: ${error.message}</td></tr>`;
    }
}

// Create a limit order
async function createLimitOrder(formData) {
    if (!localStorage.getItem('walletPubkey')) {
        showError('Please connect a wallet first');
        return;
    }
    
    showLoading('Creating limit order...');
    
    // Prepare the request data
    const requestData = {
        source_token: formData.get('source_token'),
        target_token: formData.get('target_token'),
        amount: parseFloat(formData.get('amount')),
        price_target: parseFloat(formData.get('price_target')),
        order_type: formData.get('order_type'),
        slippage: parseFloat(formData.get('slippage')),
    };
    
    // Add expiry time if provided
    const expiryTime = formData.get('expiry_time');
    if (expiryTime) {
        requestData.expiry_time = new Date(expiryTime).toISOString();
    }
    
    try {
        const response = await axios.post(`${API_URL}/set_limit_order`, requestData);
        
        if (response.data.success) {
            // Show success message
            alert('Limit order created successfully!');
            
            // Reset the form
            limitOrderForm.reset();
            
            // Refresh orders list
            await fetchOrders();
        } else {
            showError('Failed to create limit order: ' + response.data.error);
        }
    } catch (error) {
        showError('Error creating limit order: ' + error.message);
    }
    
    hideLoading();
}

// Cancel a limit order
async function cancelOrder(orderId) {
    if (confirm('Are you sure you want to cancel this order?')) {
        showLoading('Cancelling order...');
        
        try {
            const response = await axios.post(`${API_URL}/cancel_limit_order`, {
                order_id: orderId
            });
            
            if (response.data.success) {
                // Refresh orders list
                await fetchOrders();
            } else {
                showError('Failed to cancel order: ' + response.data.error);
            }
        } catch (error) {
            showError('Error cancelling order: ' + error.message);
        }
        
        hideLoading();
    }
}

// Helper: Show wallet connected UI
function showWalletConnected(pubkey) {
    walletPubkeyDisplay.textContent = `${pubkey.substring(0, 8)}...${pubkey.substring(pubkey.length - 8)}`;
    noWalletAlert.style.display = 'none';
    walletDetails.style.display = 'block';
}

// Helper: Populate token select dropdowns
function populateTokenSelects() {
    // Clear existing options
    sourceTokenSelect.innerHTML = '';
    targetTokenSelect.innerHTML = '';
    
    // Add options for each token
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
    
    // Set default values (if needed)
    sourceTokenSelect.value = knownTokens[0].mint; // SOL
    targetTokenSelect.value = knownTokens[1].mint; // USDC
    
    // Add Stop Loss option to order type dropdown
    const orderTypeSelect = document.getElementById('order-type');
    if (orderTypeSelect) {
        // Check if Stop Loss option already exists
        let hasStopLoss = false;
        for (let i = 0; i < orderTypeSelect.options.length; i++) {
            if (orderTypeSelect.options[i].value === 'StopLoss') {
                hasStopLoss = true;
                break;
            }
        }
        
        // Add StopLoss option if it doesn't exist
        if (!hasStopLoss) {
            const stopLossOption = document.createElement('option');
            stopLossOption.value = 'StopLoss';
            stopLossOption.textContent = 'Stop Loss';
            orderTypeSelect.appendChild(stopLossOption);
        }
    }
}

// Helper: Get token symbol from mint address
function getTokenSymbol(mintAddress) {
    const token = knownTokens.find(t => t.mint === mintAddress);
    return token ? token.symbol : mintAddress.substring(0, 6) + '...';
}

// Helper: Show loading modal
function showLoading(message = 'Processing request...') {
    document.getElementById('loading-message').textContent = message;
    loadingModal.show();
}

// Helper: Hide loading modal
function hideLoading() {
    loadingModal.hide();
}

// Helper: Show error message
function showError(message) {
    hideLoading();
    alert(message);
}

// Event Listeners
document.addEventListener('DOMContentLoaded', initApp);
generateWalletBtn.addEventListener('click', generateWallet);
importWalletBtn.addEventListener('click', importWallet);
refreshBalancesBtn.addEventListener('click', fetchBalances);
refreshPricesBtn.addEventListener('click', fetchPrices);
refreshOrdersBtn.addEventListener('click', fetchOrders);

limitOrderForm.addEventListener('submit', function(event) {
    event.preventDefault();
    const formData = new FormData(limitOrderForm);
    createLimitOrder(formData);
});

// Event listener for order type change
document.getElementById('order-type').addEventListener('change', function(event) {
    const orderType = event.target.value;
    const priceTargetLabel = document.querySelector('label[for="price-target"]');
    const priceTargetHelp = document.getElementById('price-target-help');
    
    // Create help text element if it doesn't exist
    if (!priceTargetHelp) {
        const helpElement = document.createElement('div');
        helpElement.id = 'price-target-help';
        helpElement.className = 'form-text';
        document.querySelector('label[for="price-target"]').parentNode.appendChild(helpElement);
    }
    
    if (orderType === 'Buy') {
        priceTargetLabel.textContent = 'Price Target (USD) - Buy when price drops to this value';
        document.getElementById('price-target-help').textContent = 'Order will execute when the price drops to or below this value';
    } else if (orderType === 'Sell') {
        priceTargetLabel.textContent = 'Price Target (USD) - Sell when price rises to this value';
        document.getElementById('price-target-help').textContent = 'Order will execute when the price rises to or above this value';
    } else if (orderType === 'StopLoss') {
        priceTargetLabel.textContent = 'Stop Loss Price (USD) - Sell when price drops to this value';
        document.getElementById('price-target-help').textContent = 'Order will execute when the price drops to or below this value to prevent further losses';
    }
}); 