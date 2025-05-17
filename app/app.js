import { validateURL } from './utils.js';

document.addEventListener('DOMContentLoaded', function() {
    const shortenButton = document.getElementById('shortenButton');
    shortenButton.addEventListener('click', shortenLink);
    function shortenLink() {
        const inputLink = document.getElementById('inputLink').value;
        if (!validateURL(inputLink)) {
            console.error('Error: input link has wrong format');
            document.getElementById('shortenedLink').textContent = "Input link has wrong format";
            return;
        }
        const data = {
            url: inputLink,
        };

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
            const shortenedLink = `${window.appConfig.NGINX_URL}/s/${data.short}`;
            document.getElementById('shortenedLink').textContent = shortenedLink;
        })
        .catch(error => {
            console.error('Error:', error);
            document.getElementById('shortenedLink').textContent = "Failed!";
        });
    }
});