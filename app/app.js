document.addEventListener('DOMContentLoaded', function () {
    const app = document.getElementById('app');
    const backendProxy = '/api';

    app.innerHTML = `
        <h1>Backend Status Checker</h1>
        <button id="statusBtn">Check Backend Status</button>
        <div id="statusResult" style="margin-top: 20px;"></div>
    `;

    const statusBtn = document.getElementById('statusBtn');
    const statusResult = document.getElementById('statusResult');

    statusBtn.addEventListener('click', async function () {
        try {
            statusResult.textContent = "Checking...";
            statusBtn.disabled = true;

            const response = await fetch(`${backendProxy}/status`);
            statusResult.innerHTML = `
                <p>Status: <strong>${response.status}</strong></p>
                <p>Status Text: <strong>${response.statusText}</strong></p>
            `;

        } catch (error) {
            statusResult.innerHTML = `
                <p style="color: red;">Error: ${error.message}</p>
            `;
        } finally {
            statusBtn.disabled = false;
        }
    });
});