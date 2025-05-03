document.addEventListener('DOMContentLoaded', function() {
    const shortenButton = document.getElementById('shortenButton');
    shortenButton.addEventListener('click', shortenLink);
    function shortenLink() {
        const inputLink = document.getElementById('inputLink').value;
        const charset = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
        let shortLink = '';
        for (let i = 0; i < 8; i++) {
            const randomIndex = Math.floor(Math.random() * charset.length);
            shortLink += charset[randomIndex];
        }
        const data = {
            old: inputLink,
            short: shortLink
        };
        let randomUrl = `${window.appConfig.NGINX_URL}/${shortLink}`;

        fetch('/api/', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify(data)
        })
        .then(response => response.json())
        .then(data => {
            console.log('Server response:', data);
            document.getElementById('shortenedLink').textContent = randomUrl;
        })
        .catch(error => {
            console.error('Error:', error);
            document.getElementById('shortenedLink').textContent = randomUrl;
        });
    }
});