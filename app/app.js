document.addEventListener('DOMContentLoaded', function() {
    const shortenButton = document.getElementById('shortenButton');
    shortenButton.addEventListener('click', shortenLink);
    function shortenLink() {
        const inputLink = document.getElementById('inputLink').value;
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
            document.getElementById('shortenedLink').textContent = `${window.appConfig.NGINX_URL}/${data.short}`;
        })
        .catch(error => {
            console.error('Error:', error);
            document.getElementById('shortenedLink').textContent = "Failed!";
        });
    }
});