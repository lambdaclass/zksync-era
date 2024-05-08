from flask import Flask
from flask import jsonify

app = Flask(__name__)

@app.route("/conversion_rate/<token_address>")
def conversion_rate(token_address):
    print(f'token address: {token_address}')
    return jsonify(123)
