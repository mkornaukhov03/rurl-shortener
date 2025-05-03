document.addEventListener('DOMContentLoaded', function() {
    const shortenButton = document.getElementById('shortenButton');
    shortenButton.addEventListener('click', shortenLink);
    function shortenLink() {
        const inputLink = document.getElementById('inputLink').value;
        const charset = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
        let randomUrl = 'click.ru/';
        for (let i = 0; i < 8; i++) {
            const randomIndex = Math.floor(Math.random() * charset.length);
            randomUrl += charset[randomIndex];
        }
        const data = {
            old: inputLink,
            short: randomUrl
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
            console.log('Ответ от сервера:', data);
            document.getElementById('shortenedLink').textContent = randomUrl;
        })
        .catch(error => {
            console.error('Ошибка:', error);
            document.getElementById('shortenedLink').textContent = randomUrl;
        });
    }
});