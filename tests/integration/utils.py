import json
import subprocess


def parse_proof(program_name):
    completed_process = subprocess.run([
        'bin/stark_parser',
        f'proofs/{program_name}.bin',
        'proof'],
        capture_output=True)
    return completed_process.stdout


def parse_public_inputs(program_name):
    pwd = subprocess.run(['pwd'], capture_output=True).stdout[:-1]
    completed_process = subprocess.run([
        'bin/stark_parser',
        f'proofs/{program_name}.bin',
        'public-inputs'],
        capture_output=True)
    return completed_process.stdout
