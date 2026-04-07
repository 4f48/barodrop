import storage
import sdcardio
import board
import busio
import digitalio
import time
import adafruit_bmp3xx

spi = busio.SPI(board.SD_CLK, board.SD_MOSI, board.SD_MISO)
sd = sdcardio.SDCard(spi, board.SD_CS)
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

i2c = board.STEMMA_I2C()
bmp = adafruit_bmp3xx.BMP3XX_I2C(i2c)

bmp.reset()
bmp.pressure_oversampling = 8
bmp.temperature_oversampling = 1
bmp.filter_coefficient = 2

while True:
    print(bmp.altitude)

