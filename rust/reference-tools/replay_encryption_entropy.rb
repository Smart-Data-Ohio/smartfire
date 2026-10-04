# Replay only the recorded random IV inputs; Rails still encrypts and serializes.
# This keeps fresh oracle output comparable across pins without substituting ciphertext.
require 'json'
require 'base64'
require 'openssl'

module ReplayEncryptionEntropy
  INPUTS = JSON.parse(File.read(File.join(__dir__, 'encryption_entropy_inputs.json')))
  KEY = :parity_encryption_iv_inputs

  module CipherInputs
    def random_iv
      inputs = Thread.current[ReplayEncryptionEntropy::KEY]
      return super unless inputs
      raise 'unexpected encryption IV draw' if inputs.empty?
      self.iv = Base64.strict_decode64(inputs.shift)
    end
  end
  OpenSSL::Cipher.prepend(CipherInputs)

  def self.with(name = nil, ivs: nil)
    raise 'nested encryption entropy replay' if Thread.current[KEY]
    Thread.current[KEY] = (ivs || INPUTS.fetch(name)).dup
    result = yield
    raise 'unused encryption IV inputs' unless Thread.current[KEY].empty?
    result
  ensure
    Thread.current[KEY] = nil
  end
end
