import time

import board
import busio
import digitalio
import sdcardio
import storage
import supervisor
from adafruit_dps310 import advanced

supervisor.runtime.autoreload = False

spi0 = busio.SPI(board.SDA, board.SCL, board.TX)
sd = sdcardio.SDCard(spi0, board.RX)
vfs = storage.VfsFat(sd)
storage.mount(vfs, "/sd")

altitude = None
timeout = None

with open("/sd/config.txt", "r") as f:
    for line in f:
        config = line.split("=")
        keyword = config[0]
        value = config[1]

        if keyword == "altitude":
            altitude = float(value)
        elif keyword == "timeout":
            timeout = float(value)
        else:
            print(f"invalid keyword: {config[0]}")


led = digitalio.DigitalInOut(board.LED)
led.direction = digitalio.Direction.OUTPUT

if altitude is None or timeout is None:
    print("error: configuration was not set")
    while True:
        for _ in range(2):
            led.value = True
            time.sleep(0.1)
            led.value = False
            time.sleep(0.1)
        time.sleep(1)

print(f"configuration: altitude={altitude}, timeout={timeout}")

i2c = busio.I2C(board.A3, board.A2)
dps310 = advanced.DPS310_Advanced(i2c)

dps310.reset()

dps310.pressure_rate = advanced.Rate.RATE_1_HZ  # type: ignore
dps310.temperature_rate = advanced.Rate.RATE_1_HZ  # type: ignore
dps310.pressure_oversample_count = advanced.SampleCount.COUNT_16  # type: ignore
dps310.temperature_oversample_count = advanced.SampleCount.COUNT_16  # type: ignore

dps310.mode = advanced.Mode.CONT_PRESTEMP  # type: ignore
dps310.wait_temperature_ready()
dps310.wait_pressure_ready()

dps310.sea_level_pressure = dps310.pressure

start = time.monotonic()

for _ in range(3):
    led.value = True
    time.sleep(0.1)
    led.value = False
    time.sleep(0.1)

while True:
    alt = dps310.altitude
    elapsed = time.monotonic() - start
    print(f"{alt} {elapsed}")
    if alt > altitude or elapsed > timeout:
        while True:
            led.value = True
            time.sleep(0.1)
            led.value = False
            time.sleep(0.1)
    time.sleep(1.0)
